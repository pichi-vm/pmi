# Motivation

PMI exists to solve four problems with how low-level virtual-machine
images are defined, launched, attested, and tooled today. This document
defines those problems; [Overview](overview.md) defines the goals that
solve them and the methods that deliver those goals.

## 1. The platform-conformance inversion

The historical pattern for booting a machine is: firmware defines the
platform layout, guest software adapts to it. The firmware enumerates
devices, decides where memory lives, exposes a CPU configuration, and
hands that picture to the kernel through a well-known interface (a
Devicetree, an ACPI table set, an `e820` map, etc.). The guest reads
the interface and configures itself accordingly.

On bare metal this asymmetry made sense. The firmware ran first, had
direct knowledge of the underlying hardware, and was structurally
positioned to discover the platform. The guest had limited capability
to express what it required in early boot, and bare-metal hardware was
largely fixed regardless of what the guest might have wanted.

Virtual machines flip the capability asymmetry.

A hypervisor has near-arbitrary flexibility to compose any platform the
guest will see — any set of virtual devices, any memory map, any CPUID
exposure, any interrupt controller version. The guest, meanwhile, has
essentially the same early-boot constraints it had on bare metal: it
cannot re-verify the platform after the fact, cannot defend itself
against subtle configuration discrepancies, and is at the mercy of
whatever the hypervisor chose to present. The party with the most
flexibility is the one whose choices the other party cannot
effectively check.

Confidential Computing extends this into a security boundary. The
hypervisor is no longer trusted, but the guest still consumes the
platform definition the hypervisor produces. A maliciously-crafted
DSDT, an unexpected MMIO region, a missing or substituted device — the
guest has no practical way to defend against these in early boot.
Concrete demonstrations of this attack surface exist:

- [AMD-SB-3012](https://www.amd.com/en/resources/product-security/bulletin/amd-sb-3012.html)
  — ACPI/AML injection in SEV guests via QEMU.
- [BadAML](https://dl.acm.org/doi/10.1145/3719027.3765123) (ACM CCS 2025,
  Distinguished Paper) — universal AML injection across SEV and TDX
  guests.

The problem: the guest has no way to constrain what the hypervisor
presents, and is too early in boot to validate it adversarially.

## 2. The artifact-sprawl problem

Linux deployers boot through more shapes than bare metal does. A
modern VM boot pipeline grows the bare-metal _firmware → kernel_
pattern into _firmware (UEFI) → hypervisor → (optional service
module) → (optional firmware) → kernel_, and the kernel itself may be
embedded in the image, loaded from disk, or extracted by the
hypervisor.

![Boot pipelines: bare metal versus modern VM](images/boot-modes.excalidraw.svg)

A **service module** is a CC-specific privileged component that
initializes the confidential environment and exposes services such as
a vTPM before dropping the guest firmware to a lower privilege level.
[COCONUT-SVSM](https://github.com/coconut-svsm/svsm) on AMD SEV-SNP is
one example; a Hyper-V–style **paravisor** loaded at the highest guest
privilege level fits the same architectural slot. Service modules are
absent from bare metal and non-CC VM boot.

Concrete examples of where real deployments land in this pipeline:

- `qemu -kernel image.efi` — the VMM extracts the kernel directly from
  the PE and starts the guest via the Linux boot protocol; no firmware
  involved.
- `qemu -bios OVMF.fd -kernel image.efi` — OVMF runs as guest UEFI,
  receives the PE over `fw_cfg`, executes its EFI stub, and boots the
  kernel from the PE.
- `qemu -bios OVMF.fd -drive file=disk.img,...` — OVMF runs as guest
  UEFI and loads the kernel from a virtual disk; the PE itself need
  not carry a kernel.
- COCONUT-SVSM + OVMF under SEV-SNP — the VMM launches the SVSM at
  VMPL0, which initializes the confidential environment, exposes a
  vTPM, and transitions OVMF to VMPL1; OVMF then boots the kernel from
  a virtual disk.
- UEFI on bare metal via PXE or HTTP Boot — firmware fetches the PE
  remotely; the EFI stub boots the kernel.

Historically each of these shapes required its own image format and
build pipeline — PE for UEFI boot, UKI for VMs that direct-boot, IGVM
(PMI's primary prior art) for paravisor-style confidential boot. An
image needing to serve more than one shape became more than one image,
with parallel build paths to maintain.

The problem: the same workload, packaged for multiple boot shapes,
fragments into incompatible artifacts.

## 3. The attestation-divergence problem

Confidential-computing launches produce a cryptographic measurement
(SEV-SNP launch digest, CCA RIM, TDX MRTD) that a remote verifier uses
to identify what was launched. The measurement is supposed to be the
identity of the workload.

Today, two different VMMs of the same target launching the same image
can produce different measurements. Several mechanisms drive the
divergence:

- **Page submission order is implementation-defined.** A VMM that
  submits pages in disk-section order versus one that submits in
  ascending GPA order will compute different incremental hashes from
  the same bytes.
- **The host picks values that contribute to the measurement.** SEV's
  CPUID page and secrets-page placeholder, CCA's `RmiRealmParams`
  (SVE vector length, debug counts, hash algorithm), TDX's
  `ATTRIBUTES` and `XFAM` — each of these gets measured into the
  cryptographic register today, but the host decides what bytes they
  contain. The same image under two different hosts produces two
  different attestations.
- **The host picks values that go into the attestation report
  alongside the cryptographic register.** SEV launch policy, TDX
  `MRCONFIGID` / `MROWNER` / `MROWNERCONFIG`, CCA RPV — even when
  these don't enter the cryptographic hash, verifier policy checks
  them, and they vary per deployer.

The result: workload reproducibility breaks at the verifier. A
verifier that wants to bind a workload to a specific attestation value
must know not just the image but which VMM is running it and which
deployer assembled the launch — defeating the point of remote
attestation as a workload-identity primitive.

The problem: a CC measurement that depends on the VMM is not a
workload identity; it's a launch-event identity.

## 4. The tooling-fragmentation problem

A new image format historically demands a new toolchain at every
layer:

- **Producers.** A separate builder per format (PE-builder for UEFI,
  UKI-builder for direct-boot VMs, IGVM-builder for paravisor-style
  confidential boot). Image authors shipping to multiple shapes
  maintain parallel build pipelines and reconcile their outputs by
  hand.
- **Consumers.** Each VMM rolls its own image parser. A bug found in
  one parser doesn't fix the others; a hardening pass in one doesn't
  harden the others.
- **In-guest stubs.** Each boot shape has its own pre-kernel
  component (PE EFI stub, UKI loader, IGVM boot shim). They overlap
  significantly but don't share code, because their input formats
  don't share structure.
- **Verifiers.** Each cryptographic-measurement protocol gets its own
  measurement-reproduction tool. Cross-target verifier libraries are
  rare; cross-VMM verifier libraries are rarer.
- **Inspectors.** "Show me what this image will do" is hard to answer
  with a single tool when the image's format depends on the boot path
  the deployer happens to take.
- **Signers and verifiers of identity.** UEFI Secure Boot signs PE;
  IGVM has its own signature; UKI has its own conventions. A tenant
  signing for multiple shapes signs multiple artifacts.

Existing PE-based tooling (`objcopy`, `objdump`, `sbsign`, `pesign`,
`systemd-ukify`, `systemd-stub`, UEFI loaders) already covers the
PE-format work for one shape (bare-metal UKI). A new image format
that abandons PE forces all of these tools to be replaced or
reinvented; one that extends PE without breaking the existing
conventions inherits the existing ecosystem.

New tools that are PMI-specific (target-spec parsers, DTBO mergers,
builders, VMMs, in-guest consumers, verifiers, signers, inspectors)
each want to be small and focused enough that they compose across
contexts: the same parser inside a builder and inside a VMM and
inside a verifier; the same DTBO applier inside a bootloader stub
and inside a future kernel-side merger.

The problem: every new shape doubles the toolchain surface area
unless the format is designed to inherit existing tooling and to
keep its new tooling narrow and composable.

---

The four goals PMI sets to solve these problems, and the methods that
deliver them, are in [Overview](overview.md).
