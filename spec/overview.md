# Overview

## What Needs to Be Loaded

Booting a machine requires loading software into memory. What gets loaded
depends on the deployment context:

- **Bare metal:** A kernel, initial ramdisk (initrd), and command line are
  loaded into memory and executed. On UEFI systems, a bootloader or EFI
  application handles this.

- **Virtual machine:** A Virtual Machine Monitor (VMM) loads a kernel into guest
  memory, or loads guest firmware such as OVMF (Open Virtual Machine Firmware)
  which then loads the kernel from disk. The VMM may also need to provide
  runtime data like a memory map or ACPI tables.

- **Confidential VM:** Everything a VM needs, plus platform-specific pages that
  the hardware requires: initial register state, secrets pages, CPUID tables. A
  service module may run at a higher privilege level than the guest firmware.
  The VMM loads all of this into guest memory, in the correct order, and feeds
  each page to the platform's measurement API. The VMM is untrusted — hardware
  attestation allows a remote verifier to confirm that the VMM loaded exactly
  what the image specified.

Today, each context uses a different image format with different tooling. PMI
(Portable Machine Image) uses a single PE binary to address all three contexts.

## How PE Works

PE (Portable Executable) is the binary format that UEFI firmware loads and
executes. A PE file contains executable code (the entry point that UEFI calls)
and is divided into **sections**, each with a name and a set of attributes
defined in the section header:

| Field              | Description                                        |
| ------------------ | -------------------------------------------------- |
| `Name`             | Up to 8 bytes (e.g., `.text`, `.data`, `.reloc`)   |
| `VirtualAddress`   | Guest physical address where the section is loaded |
| `VirtualSize`      | How much memory the section occupies               |
| `SizeOfRawData`    | How much data is stored on disk                    |
| `PointerToRawData` | Offset of the on-disk data within the file         |
| `Characteristics`  | Flags that control how the section is treated      |

When UEFI firmware loads a PE, it reads each section header and copies the
on-disk data into memory at the section's `VirtualAddress`. If `VirtualSize` is
larger than `SizeOfRawData`, the remainder is zero-filled (this is how `.bss`
regions work — reserved memory with no file backing).

Not all sections are loaded. Sections can be marked with flags like
`IMAGE_SCN_MEM_DISCARDABLE` that tell the loader to skip them. These sections
exist in the file but are not mapped into memory — they carry metadata that the
loader does not need at runtime.

## How UKI Uses PE Sections

A Unified Kernel Image (UKI) is a PE file that bundles everything needed to boot
Linux into named sections:

| Section    | Contents                |
| ---------- | ----------------------- |
| `.linux`   | The kernel (bzImage)    |
| `.initrd`  | The initial ramdisk     |
| `.cmdline` | The kernel command line |
| `.osrel`   | OS release metadata     |

The PE also contains an **EFI stub** — a small program that serves as the PE's
entry point. When UEFI firmware loads the PE, it calls the stub. The stub reads
the other sections, loads the kernel into memory, and boots it. On bare metal,
PXE, and UEFI HTTP Boot, no additional configuration is required.

Sections like `.osrel` are not loaded by UEFI — they are non-loaded metadata
that the stub or other tools can read. This is how UKI carries data beyond what
the loader itself needs.

VMMs can also boot a UKI without bare metal hardware. There are two methods:

- **Extracted:** The VMM extracts the kernel from the PE and loads it directly
  into guest memory using the Linux boot protocol, with no guest firmware
  involved (e.g., `qemu -kernel image.efi`). Confidential computing VMs can be
  built this way.

- **Stubbed:** The VMM loads a guest UEFI implementation (e.g., OVMF) and passes
  the UKI to it via the `fw_cfg` interface. OVMF then boots the UKI through its
  normal UEFI boot path (e.g., `qemu -bios OVMF.fd -kernel image.efi`). This is
  the method used by the AMD SEV variant of OVMF (along with a measurement page
  to ensure that the VMM feeds the correct UKI to the SEV platform).

However, in current deployments, VMMs supply full, unmeasured ACPI tables,
including executable AML bytecode. This has been demonstrated as a practical
attack surface allowing root compromise of confidential VMs:

- [AMD-SB-3012](https://www.amd.com/en/resources/product-security/bulletin/amd-sb-3012.html)
  — ACPI/AML injection in SEV guests via QEMU.
- [BadAML](https://dl.acm.org/doi/10.1145/3719027.3765123) (ACM CCS 2025,
  Distinguished Paper) — universal AML injection across SEV and TDX guests.

This is precisely the kind of vulnerability that IGVM/PMI aims to eliminate by
allowing the image to specify exactly what gets loaded and measured.

## Extending PE/UKI for VMMs

UKI uses PE sections to carry data beyond what UEFI firmware needs. PMI extends
this model by adding a single non-loaded PE section: `.pmi`.

The `.pmi` section contains a CBOR (Concise Binary Object Representation)
encoded **[manifest](manifest/README.md)**. The manifest references other PE
sections by name and tells the VMM what to do with them:

- Which sections to load into guest memory, and in what order.
- Which sections are platform-specific (e.g., only needed on AMD SEV).
- Which sections the VMM should fill with generated data (e.g., a memory map).
- What platform policy to apply (e.g., SEV launch policy).

PMI does not define the additional PE sections themselves — they can contain
anything (firmware, service modules, platform-specific pages, etc.) as long as
they are named in the manifest and follow PMI's
[alignment rules](pe.md#page-granularity). The image author
decides what sections to include; the manifest tells the VMM how to use them.

A VMM that understands PMI reads the `.pmi` section, parses the manifest, and
follows its instructions. A VMM that does not understand PMI boots the image as
a standard UKI. UEFI firmware skips the `.pmi` section entirely.

The manifest is not integrity-protected by PMI itself. However, the manifest's
instructions determine what data is loaded and in what order, which directly
affects the platform's launch measurement. Any change to the manifest that
alters what is loaded will produce a different measurement and be visible in the
attestation report.

## Example: What a PMI Image Contains

A PMI image for a serviced confidential VM might contain:

| Section    | Loaded by UEFI? | Purpose                                   |
| ---------- | --------------- | ----------------------------------------- |
| `.linux`   | Yes (via stub)  | Kernel                                    |
| `.initrd`  | Yes (via stub)  | Initial ramdisk                           |
| `.cmdline` | Yes (via stub)  | Kernel command line                       |
| `.ovmf`    | No              | Guest firmware, loaded by VMM             |
| `.sev.svm` | No              | SVSM service module, loaded by VMM on SEV |
| `.sev.vms` | No              | VMSA register state, loaded by VMM on SEV |
| `.sev.sec` | No              | Secrets page, populated by platform       |
| `.sev.cpu` | No              | CPUID page, populated by VMM on SEV       |
| `.memmap`  | No              | EFI memory map, filled by VMM             |
| `.pmi`     | No              | Manifest (CBOR)                           |

On bare metal, UEFI executes the EFI stub, which boots the kernel from `.linux`.
The non-loaded sections are ignored.

In a VM, the VMM reads `.pmi`, loads the sections the manifest specifies, fills
in the memory map, and starts the guest.

In a confidential VM running in SEV, the VMM additionally loads
platform-specific sections (`.sev.svm`, `.sev.vms`, `.sev.sec`, `.sev.cpu`),
applies the launch policy, and measures everything through the platform's
hardware APIs.

## PE Constraints and Page Granularity

PMI requires the manifest in a non-loaded `.pmi` PE section, limits section
names to 8 bytes, and imposes alignment rules that enable zero-copy loading
with 2M huge pages. See [PE constraints and page granularity](pe.md) for the
full requirements.

## VMM Execution Model

The VMM processes the manifest in eight steps:

1. **Select platform.** Identify the current CC platform (or `"native"`).
2. **Merge policy.** Merge image and deployer [policy](manifest/policy.md).
3. **Platform initialize.** Initialize the platform's cryptographic context.
4. **Platform pre-load.** Execute platform-specific pre-section actions.
5. **Process [sections](manifest/sections.md).** Load, fill, or skip each
   section in array order. Measure as appropriate.
6. **Platform post-load.** Execute platform-specific post-section actions.
7. **Platform finalize.** Seal the measurement.
8. **Start the guest.**

Section order is security-critical: on CC platforms, the measurement is an
ordered hash chain, so reordering sections produces a different digest. See
[sections](manifest/sections.md) for the full loading and measurement rules,
and each [platform binding](manifest/platforms/) for the platform-specific API
mapping.
