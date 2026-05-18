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

Historically each shape required its own image format and build pipeline —
PE for UEFI boot, UKI for VMs that direct-boot, IGVM for paravisor-style
confidential boot. An image that needed to serve more than one deployment
shape became more than one image, with parallel build paths to maintain.

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
- Schemas are exhaustive: a reference parser can decide a target spec is
  valid or invalid with no third answer.
- On-disk alignment supports zero-copy loading with huge pages.

## Non-goals

- PMI does not define a new container. PMI is PE.
- PMI does not define cross-target inheritance, fallback, or filtering.
  Each target's spec is self-contained.
- PMI does not provide an out-of-band vendor extension mechanism. The type
  namespace is closed; the spec evolves through versioned revisions.
- PMI does not specify deployer-supplied launch inputs (`host_data`,
  policy overrides, etc.). Those are VMM CLI concerns, out of scope here.
- PMI does not replace IGVM in the narrower problem IGVM solves well.

## Context

### Boot modes Linux supports

A machine boots by combining three components, each of which may be absent,
provided by the host, provided by the tenant (bundled in the image), or
loaded from disk:

| Mode        | Service |    Firmware     |  Kernel   | BM  | VM  | CVM |
| :---------- | :-----: | :-------------: | :-------: | --- | --- | --- |
| Extracted   |         |                 | extracted |     | ✓   | ✓   |
| Stubbed     |         | vm: yes, bm: no |  stubbed  | ✓   | ✓   | ✓   |
| Traditional |         |       yes       |  on disk  |     | ✓   | ✓   |
| Serviced    |   yes   |       yes       |  on disk  |     |     | ✓   |

1. **Extracted** — VM only. The VMM takes the role of guest firmware,
   extracts the kernel from the PE, and starts the guest via the Linux
   boot protocol. Example: `qemu -kernel image.efi`.

2. **Stubbed** — bare metal or VM. UEFI executes the PE. The PE contains
   an EFI stub and a kernel (UKI shape); the stub loads the kernel into
   memory. Works on bare metal via PXE / HTTP Boot. In a VM, requires a
   guest UEFI implementation (e.g., OVMF).

3. **Traditional** — bare metal or VM. UEFI executes the PE, but the PE
   carries no kernel — only firmware and boot configuration. The firmware
   loads the kernel from disk.

4. **Serviced** — CVM only. The tenant bundles a service module and
   firmware. The VMM launches the service module at the privileged layer;
   it initializes the confidential environment and launches the tenant's
   firmware, which boots the kernel and measures it via a vTPM the service
   layer exposes. Example: COCONUT-SVSM + OVMF.

A **service module** is a CC-specific privileged component (e.g.,
[COCONUT-SVSM](https://github.com/coconut-svsm/svsm) on AMD SEV-SNP) that
initializes the confidential environment and exposes a vTPM before dropping
the guest firmware to a lower privilege level. Service modules are absent
from bare metal and non-CC VM boot.

### Existing formats

| Mode        | PE  | UKI | IGVM | PMI |
| :---------- | :-: | :-: | :--: | :-: |
| Extracted   |     |  ✓  |  ✓   |  ✓  |
| Stubbed     |  ✓  |  ✓  |      |  ✓  |
| Traditional |     |     |  ✓   |  ✓  |
| Serviced    |     |     |  ✓   |  ✓  |

PMI is a strict superset of PE. A PMI image MAY also be UKI-shaped — that's
a content choice, not a requirement of PMI.

### Relation to IGVM

PMI is inspired by IGVM, which addresses the same family of problems for a
narrower scope (loading paravisor-style confidential guest images for
specific VMMs). PMI's broader scope drove several design choices that
differ from IGVM's:

- PMI is a PE, so the same artifact boots through UEFI, PXE, and HTTP Boot
  paths in addition to VM and CVM paths.
- PMI describes regions rather than per-page load commands, allowing
  zero-copy `mmap()` loading and trivial use of huge pages.
- PMI reuses standard interfaces (Devicetree for platform topology) rather
  than defining VMM-specific structures for the same information.
- PMI separates the image's launch recipe from the deployer's policy
  inputs, so the same image can be deployed with different external
  configuration without rebuilding.
- PMI images are inspectable and modifiable with standard PE tooling.

For the narrower problem IGVM was designed for, IGVM remains the
better-fitting tool. PMI exists to cover the broader set of deployment
shapes Linux ecosystems actually use.
