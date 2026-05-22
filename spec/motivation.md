# Motivation

PMI exists to solve four problems. Each problem has a one-to-one
corresponding goal in [Overview](overview.md). This document defines
the problems and explains why they are problems; the overview defines
the goals that solve them and the methods that deliver those goals.

| # | Problem (this document)                                | Goal (overview.md)                                                                                |
| - | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| 1 | Early boot can't defend against hypervisor attacks     | [Security against a malicious hypervisor](overview.md#security-against-a-malicious-hypervisor)    |
| 2 | One workload needs multiple image artifacts            | [Executable format portability](overview.md#executable-format-portability)                        |
| 3 | Same image, different VMM, different attestation       | [Attestation equivalence](overview.md#attestation-equivalence)                                    |
| 4 | Every new image format needs new tools at every layer  | [Tooling reuse](overview.md#tooling-reuse)                                                        |

## 1. Early boot can't defend against hypervisor attacks

On bare metal, firmware defines the platform layout and the guest
adapts to it: firmware enumerates devices, decides where memory lives,
exposes a CPU configuration, and hands the picture to the kernel
through a well-known interface (Devicetree, ACPI, `e820`). The
asymmetry is structurally justified — firmware has direct knowledge of
the underlying hardware and runs first; the guest has limited
early-boot capability and hardware is largely fixed regardless of what
the guest wants.

Virtual machines invert the capability asymmetry. A hypervisor has
near-arbitrary flexibility to compose any platform the guest will
see — any set of virtual devices, any memory map, any CPUID exposure,
any interrupt controller version. The guest, meanwhile, keeps the
same early-boot constraints it had on bare metal: it cannot re-verify
the platform after the fact, cannot defend itself against subtle
configuration discrepancies, and is at the mercy of whatever the
hypervisor chose to present. The party with the most flexibility is
the one whose choices the other party cannot effectively check.

Confidential Computing extends the asymmetry into a security boundary.
The hypervisor is no longer trusted, but the guest still consumes the
platform definition the hypervisor produces. A maliciously-crafted
DSDT, an unexpected MMIO region, a missing or substituted device — the
guest has no practical way to defend against any of these in early
boot. The attack surface is concrete and demonstrated:

- [AMD-SB-3012](https://www.amd.com/en/resources/product-security/bulletin/amd-sb-3012.html)
  — ACPI/AML injection in SEV guests via QEMU.
- [BadAML](https://dl.acm.org/doi/10.1145/3719027.3765123) (ACM CCS
  2025, Distinguished Paper) — universal AML injection across SEV and
  TDX guests.

**PMI's response:**
[Security against a malicious hypervisor](overview.md#security-against-a-malicious-hypervisor).
The image declares the platform it expects; the VMM either provides
exactly that or refuses to launch, before the guest sees a single
byte. The guest's residual validation surface is reduced to a small,
well-known set of rules the consumer can audit end-to-end.

## 2. One workload needs multiple image artifacts

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

Real deployments span the pipeline:

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

Historically each shape required its own image format and build
pipeline — PE for UEFI boot, UKI for VMs that direct-boot, IGVM (PMI's
primary prior art) for paravisor-style confidential boot. An image
needing to serve more than one shape became more than one image, with
parallel build paths to maintain and reconcile.

**PMI's response:**
[Executable format portability](overview.md#executable-format-portability).
A single PE binary carries content for every boot shape the image
author chooses to support. Standard PE loaders ignore the parts they
don't understand, and conformant VMMs read only the target sections
relevant to them.

## 3. Same image, different VMM, different attestation

Confidential-computing launches produce a cryptographic measurement
(SEV-SNP launch digest, CCA RIM, TDX MRTD) that a remote verifier uses
to identify what was launched. The measurement is supposed to be the
identity of the workload — but today it depends on more than the
workload.

Several mechanisms drive divergence between two VMMs of the same
target running the same image:

- **Page submission order is implementation-defined.** A VMM that
  submits pages in disk-section order versus one that submits in
  ascending GPA order computes a different incremental hash from the
  same bytes.
- **The host picks values that are measured into the cryptographic
  register.** SEV's CPUID page and secrets-page placeholder, CCA's
  `RmiRealmParams` (SVE vector length, debug counts, hash algorithm),
  TDX's `ATTRIBUTES` and `XFAM` — each of these is measured today, but
  the host decides what bytes they contain. The same image under two
  different hosts produces two different measurements.
- **The host picks values that appear elsewhere in the attestation
  report.** SEV launch policy, TDX `MRCONFIGID` / `MROWNER` /
  `MROWNERCONFIG`, CCA RPV — even when these don't enter the
  cryptographic hash, verifier policy checks them, and they vary per
  deployer.

The result: workload reproducibility breaks at the verifier. A
verifier that wants to bind a workload to a specific attestation value
must know not just the image but which VMM is running it and which
deployer assembled the launch — defeating the point of remote
attestation as a workload-identity primitive.

**PMI's response:**
[Attestation equivalence](overview.md#attestation-equivalence). Every
value that contributes to image and platform identity is bound by the
PMI image; ordering of measured submissions is normatively pinned.
The same image produces bit-identical image+platform identity fields
across any two conformant VMMs of the same target.

## 4. Every new image format needs new tools at every layer

A new image format historically demands a new toolchain at every
layer:

- **Producers.** A separate builder per format (PE for UEFI, UKI for
  direct-boot VMs, IGVM for paravisor-style confidential boot). Image
  authors shipping to multiple shapes maintain parallel build
  pipelines and reconcile their outputs by hand.
- **Consumers.** Each VMM rolls its own image parser. A bug fixed in
  one parser doesn't fix the others; a hardening pass in one doesn't
  harden the others.
- **In-guest stubs.** Each boot shape has its own pre-kernel component
  (PE EFI stub, UKI loader, IGVM boot shim). They overlap
  significantly but don't share code because their input formats don't
  share structure.
- **Verifiers.** Each measurement protocol gets its own
  measurement-reproduction tool. Cross-target verifier libraries are
  rare; cross-VMM verifier libraries are rarer.
- **Inspectors.** "Show me what this image will do" is hard to answer
  with a single tool when the image's format depends on the boot path
  the deployer happens to take.
- **Signers.** UEFI Secure Boot signs PE; IGVM has its own signature;
  UKI has its own conventions. A tenant signing for multiple shapes
  signs multiple artifacts.

Existing PE-based tooling (`objcopy`, `objdump`, `sbsign`, `pesign`,
`systemd-ukify`, `systemd-stub`, UEFI loaders) already covers the
PE-format work for one shape. A new image format that abandons PE
forces all of these tools to be replaced; one that extends PE without
breaking the existing conventions inherits the existing ecosystem.

**PMI's response:**
[Tooling reuse](overview.md#tooling-reuse). PMI extends PE without
breaking existing PE-based tools; new PMI-specific tools (target-spec
parsers, DTBO mergers, builders, VMMs, in-guest consumers, verifiers,
signers, inspectors) have narrow, target-isolated contracts so they
compose across contexts unchanged.
