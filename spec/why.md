# Why PMI?

PMI exists to solve two problems with the way low-level virtual machine
images are defined and launched today.

## The problem

### 1. The platform-definition inversion

The historical pattern for booting a machine is: firmware defines the
platform layout, guest software adapts to it. The firmware enumerates
devices, decides where memory lives, exposes a CPU configuration, and hands
that picture to the kernel through a well-known interface (a Devicetree, an
ACPI table set, an `e820` map, etc.). The guest reads the interface and
configures itself accordingly.

On bare metal this asymmetry made sense. The firmware ran first, had direct
knowledge of the underlying hardware, and was structurally positioned to
discover the platform. The guest had limited capability to express what it
required in early boot, and bare-metal hardware was largely fixed regardless
of what the guest might have wanted.

Virtual machines flip the capability asymmetry.

A hypervisor has near-arbitrary flexibility to compose any platform the
guest will see — any set of virtual devices, any memory map, any CPUID
exposure, any interrupt controller version. The guest, meanwhile, has
essentially the same early-boot constraints it had on bare metal: it cannot
re-verify the platform after the fact, cannot defend itself against subtle
configuration discrepancies, and is at the mercy of whatever the hypervisor
chose to present. The party with the most flexibility is the one whose
choices the other party cannot effectively check.

Confidential Computing extends this into a security boundary. The hypervisor
is no longer trusted, but the guest still consumes the platform definition
the hypervisor produces. A maliciously-crafted DSDT, an unexpected MMIO
region, a missing or substituted device — the guest has no practical way to
defend against these in early boot. Concrete demonstrations of this attack
surface exist:

- [AMD-SB-3012](https://www.amd.com/en/resources/product-security/bulletin/amd-sb-3012.html)
  — ACPI/AML injection in SEV guests via QEMU.
- [BadAML](https://dl.acm.org/doi/10.1145/3719027.3765123) (ACM CCS 2025,
  Distinguished Paper) — universal AML injection across SEV and TDX guests.

### 2. The single-artifact problem

Linux deployers boot through more shapes than bare metal does. A modern VM
boot pipeline grows the bare-metal _firmware → kernel_ pattern into
_firmware (UEFI) → hypervisor → (optional service module) → (optional
firmware) → kernel_, and the kernel itself may be embedded in the image,
loaded from disk, or extracted by the hypervisor.

![Boot pipelines: bare metal versus modern VM](images/boot-modes.excalidraw.svg)

A **service module** is a CC-specific privileged component that initializes
the confidential environment and exposes services such as a vTPM before
dropping the guest firmware to a lower privilege level.
[COCONUT-SVSM](https://github.com/coconut-svsm/svsm) on AMD SEV-SNP is one
example; a Hyper-V–style **paravisor** loaded at the highest guest
privilege level fits the same architectural slot. Service modules are
absent from bare metal and non-CC VM boot.

Concrete examples of where real deployments land in this pipeline:

- `qemu -kernel image.efi` — the VMM extracts the kernel directly from the
  PE and starts the guest via the Linux boot protocol; no firmware
  involved.
- `qemu -bios OVMF.fd -kernel image.efi` — OVMF runs as guest UEFI,
  receives the PE over `fw_cfg`, executes its EFI stub, and boots the
  kernel from the PE.
- `qemu -bios OVMF.fd -drive file=disk.img,...` — OVMF runs as guest UEFI
  and loads the kernel from a virtual disk; the PE itself need not carry a
  kernel.
- COCONUT-SVSM + OVMF under SEV-SNP — the VMM launches the SVSM at VMPL0,
  which initializes the confidential environment, exposes a vTPM, and
  transitions OVMF to VMPL1; OVMF then boots the kernel from a virtual
  disk.
- UEFI on bare metal via PXE or HTTP Boot — firmware fetches the PE
  remotely; the EFI stub boots the kernel.

Historically each of these shapes required its own image format and build
pipeline — PE for UEFI boot, UKI for VMs that direct-boot, IGVM (PMI's
primary prior art) for paravisor-style confidential boot. An image needing
to serve more than one shape became more than one image, with parallel
build paths to maintain.

## Goals

- The image declares the platform layout it requires; the VMM conforms or
  refuses to launch.
- A single PE binary carries the content for whichever boot shapes the
  image author wants to support — bare metal, non-CC VM, and confidential
  VM across multiple CC targets.
- Standard PE tooling (`objcopy`, `sbsign`, `systemd-ukify`, etc.) works on
  PMI images unmodified.
- Confidential-computing launch produces a deterministic, ordered
  measurement a remote verifier can bind to.
- On-disk alignment supports zero-copy loading with huge pages.

## Non-goals

- PMI does not define a new container. PMI is PE.
- PMI does not define cross-target inheritance, fallback, or filtering.
  Each target's spec is self-contained.
- PMI does not provide an out-of-band vendor extension mechanism. The type
  namespace is closed; the spec evolves through versioned revisions.
- PMI does not specify deployer-supplied launch inputs (`host_data`,
  policy overrides, etc.). Those are VMM CLI concerns, out of scope here.
