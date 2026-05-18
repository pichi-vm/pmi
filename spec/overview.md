# Overview

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

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
exist in the file but are not mapped into memory — they carry data that the
loader does not need at runtime.

## How UKI Uses PE Sections

A Unified Kernel Image (UKI) is a PE file that bundles everything needed to
boot Linux into named sections:

| Section    | Contents                |
| ---------- | ----------------------- |
| `.linux`   | The kernel (bzImage)    |
| `.initrd`  | The initial ramdisk     |
| `.cmdline` | The kernel command line |
| `.osrel`   | OS release metadata     |

The PE also contains an **EFI stub** — a small program that serves as the PE's
entry point. When UEFI firmware loads the PE, it calls the stub. The stub
reads the other sections, loads the kernel into memory, and boots it. On bare
metal, PXE, and UEFI HTTP Boot, no additional configuration is required.

VMMs can also boot a UKI without bare metal hardware. There are two methods:

- **Extracted:** The VMM extracts the kernel from the PE and loads it directly
  into guest memory using the Linux boot protocol, with no guest firmware
  involved (e.g., `qemu -kernel image.efi`).

- **Stubbed:** The VMM loads a guest UEFI implementation (e.g., OVMF) and
  passes the UKI to it via the `fw_cfg` interface. OVMF then boots the UKI
  through its normal UEFI boot path.

However, in current deployments, VMMs supply full, unmeasured ACPI tables,
including executable AML bytecode. This has been demonstrated as a practical
attack surface allowing root compromise of confidential VMs:

- [AMD-SB-3012](https://www.amd.com/en/resources/product-security/bulletin/amd-sb-3012.html)
  — ACPI/AML injection in SEV guests via QEMU.
- [BadAML](https://dl.acm.org/doi/10.1145/3719027.3765123) (ACM CCS 2025,
  Distinguished Paper) — universal AML injection across SEV and TDX guests.

This is precisely the kind of vulnerability PMI aims to eliminate by allowing
the image to specify exactly what gets loaded and measured.

## Platforms

PMI defines one **platform** per launch path the image supports. A platform
is a self-contained CBOR spec carried in its own PE section (named by
convention `.pmi.<plat>`). A VMM targeting a platform reads that platform's
section, ignores the others, and executes the recipe it finds there.

The currently defined platforms are:

| Platform        | PE section | Notes                                  |
| --------------- | ---------- | -------------------------------------- |
| [`vm`](vm.md)   | `.pmi.vm`  | Non-CC virtual machines                |
| [`sev`](sev.md) | `.pmi.sev` | AMD SEV 3.0 (SEV-SNP) confidential VMs |
| [`tdx`](tdx.md) | `.pmi.tdx` | Intel TDX confidential VMs (TODO)      |
| [`cca`](cca.md) | `.pmi.cca` | Arm CCA confidential VMs (TODO)        |

Platforms are independent — they share conventions (the [`dtb`](dtb.md) field;
the [`load`](load.md) and [`dtbo`](dtbo.md) actions) but each one fully
specifies its own launch recipe. There is no inheritance, no fallback, no
selection logic beyond "the VMM targeting `sev` reads `.pmi.sev`."

## Shape of a platform spec

Every platform spec is a CBOR map with the same outer shape:

```cddl
platform = {
  "version" => uint,                ; schema version
  ? "dtb"   => tstr,                ; PE section containing the base DTB
  "actions" => [+ action],          ; ordered launch recipe
  * tstr => any,                    ; unknown keys ignored
}
```

Each platform defines its own set of `action` types. Common action types
(used by multiple platforms) are:

- [`load`](load.md) — load a PE section's bytes into guest memory.
- [`dtbo`](dtbo.md) — VMM writes a host-decided devicetree overlay into the
  named section.

Platform-specific action types are defined by each platform binding (e.g.,
`vcpu` on `vm`, `sev:policy` / `sev:id-block` / `sev:vmsa` / ... on `sev`).

Action `type` values use the `<platform>:<name>` convention when scoped
(e.g., `sev:vmsa`); short, unscoped names (`load`, `dtbo`, `vcpu`) are used
where collisions are not a concern.

## VMM execution model

1. **Select platform.** Identify the target platform and read its PE section
   (e.g., `.pmi.sev` for SEV). If the section is absent, refuse to launch.
2. **Inspect DTB.** If the spec includes a [`dtb`](dtb.md), parse its FDT and
   validate that the host can satisfy every hardware capability it declares.
   Fail the launch if any declaration cannot be satisfied.
3. _(reserved)_
4. **Platform initialize.** Initialize the platform's cryptographic context,
   consuming any action whose type binds to this step (e.g., `sev:policy`).
5. _(reserved)_
6. **Process actions.** Process each action in array order. Each action's
   `type` selects how the VMM consumes it; common types load PE bytes into
   guest memory and are measured by the platform's measurement API as
   appropriate.
7. _(reserved)_
8. **Platform finalize.** Consume launch-finalize actions (e.g.,
   `sev:id-block` and `sev:id-auth`) and seal the measurement.
9. **Start the guest.**

Action order is security-critical on CC platforms: the launch measurement is
an ordered hash chain, so reordering actions produces a different digest.

## Example: what a PMI image contains

A PMI image supporting both `vm` and SEV serviced boot might contain the
following PE sections. Only the `.pmi.<plat>` names are used by PMI to
discover platform specs; all other names shown are illustrative.

| Section    | Loaded by UEFI? | Purpose                                    |
| ---------- | --------------- | ------------------------------------------ |
| `.linux`   | Yes (via stub)  | Kernel                                     |
| `.initrd`  | Yes (via stub)  | Initial ramdisk                            |
| `.cmdline` | Yes (via stub)  | Kernel command line                        |
| `.dtb.vm`  | No              | Base DTB used by the `vm` spec             |
| `.dtb.sev` | No              | Base DTB used by the `sev` spec            |
| `.dtbo`    | No              | Host-filled DTB overlay (memory/cpus/numa) |
| `.ovmf`    | No              | Guest firmware                             |
| `.sev.svm` | No              | SVSM service module                        |
| `.sev.vms` | No              | SEV VMSA register state                    |
| `.sev.sec` | No              | SEV secrets page                           |
| `.sev.cpu` | No              | SEV CPUID page                             |
| `.sev.idb` | No              | SEV ID block                               |
| `.sev.ida` | No              | SEV ID auth info                           |
| `.vcpu`    | No              | Boot vCPU register state for `vm`          |
| `.pmi.vm`  | No              | `vm` platform spec                         |
| `.pmi.sev` | No              | `sev` platform spec                        |

On bare metal, UEFI executes the EFI stub, which boots the kernel from
`.linux`. All `.pmi.*` and other non-loaded PE sections are ignored.

A VMM targeting `vm` reads `.pmi.vm`, inspects its `dtb`, validates
conformance, processes its `actions` (load segments, write the overlay, set
the boot vCPU), and starts the guest.

A VMM targeting `sev` reads `.pmi.sev`. Its actions drive `SNP_LAUNCH_START`
(`sev:policy`, or policy embedded in the signed `sev:id-block`),
`SNP_LAUNCH_UPDATE` (`load` and `sev:vmsa`/`sev:secrets`/`sev:cpuid`), and
`SNP_LAUNCH_FINISH` (`sev:id-block` + `sev:id-auth`), with the launch digest
covering everything fed to the platform's measurement API.

## PE constraints and page granularity

PMI imposes alignment rules on PE sections that allow zero-copy loading with
2M huge pages, and requires that platform-spec sections be non-loaded. See
[PE constraints and page granularity](pe.md) for the full rules.
