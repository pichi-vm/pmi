# Overview

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

A PMI image is a PE binary that carries, for each launch target it
supports, a self-contained CBOR spec naming the actions a VMM must perform
to launch the guest. This document is the entry point to the normative
spec: it introduces the format's mental model and points at the per-topic
reference docs. For the motivation behind PMI's existence, see
[Motivation](motivation.md).

## How PE works

PE (Portable Executable) is the binary format that UEFI firmware loads and
executes. A PE file contains executable code (the entry point that UEFI
calls) and is divided into **sections**, each with a name and a set of
attributes defined in the section header:

| Field              | Description                                        |
| ------------------ | -------------------------------------------------- |
| `Name`             | Up to 8 bytes (e.g., `.text`, `.data`, `.reloc`)   |
| `VirtualAddress`   | Guest physical address where the section is loaded |
| `VirtualSize`      | How much memory the section occupies               |
| `SizeOfRawData`    | How much data is stored on disk                    |
| `PointerToRawData` | Offset of the on-disk data within the file         |
| `Characteristics`  | Flags that control how the section is treated      |

When UEFI firmware loads a PE, it reads each section header and copies the
on-disk data into memory at the section's `VirtualAddress`. If `VirtualSize`
is larger than `SizeOfRawData`, the remainder is zero-filled (this is how
`.bss` regions work — reserved memory with no file backing).

Not all sections are loaded. Sections can be marked with flags like
`IMAGE_SCN_MEM_DISCARDABLE` that tell the loader to skip them. These
sections exist in the file but are not mapped into memory — they carry data
that the loader does not need at runtime.

## How UKI uses PE sections

A Unified Kernel Image (UKI) is a PE file that bundles everything needed to
boot Linux into named sections:

| Section    | Contents                |
| ---------- | ----------------------- |
| `.linux`   | The kernel (bzImage)    |
| `.initrd`  | The initial ramdisk     |
| `.cmdline` | The kernel command line |
| `.osrel`   | OS release metadata     |

The PE also contains an **EFI stub** — a small program that serves as the
PE's entry point. When UEFI firmware loads the PE, it calls the stub. The
stub reads the other sections, loads the kernel into memory, and boots it.
PMI builds on this same PE-with-named-sections idiom.

## PMI as a PE extension

A PMI image is a PE binary. It MAY also be structured as a UKI (carrying
`.linux`, `.initrd`, `.cmdline`, and an EFI stub) for bare-metal and
stubbed VM paths; UEFI ignores the PMI-specific sections. A PMI image is
not _required_ to be UKI-shaped — an image that contains only firmware
(for OVMF-loads-kernel-from-disk modes), or only confidential-VM content,
is equally valid. PMI is compatible with UKI, not a flavor of it.

PMI's extension to PE is a set of non-loaded sections whose names begin
with `.pmi.` — one per launch target the image supports.

## Targets

PMI defines one **target** per launch path the image supports. A target is
a self-contained CBOR spec carried in its own PE section (named by
convention `.pmi.<target>`). A VMM targeting one of them reads that
target's section, ignores the others, and executes the recipe it finds
there.

The currently defined targets are:

| Target          | PE section | Notes                                  |
| --------------- | ---------- | -------------------------------------- |
| [`vm`](vm.md)   | `.pmi.vm`  | Non-CC virtual machines                |
| [`sev`](sev.md) | `.pmi.sev` | AMD SEV 3.0 (SEV-SNP) confidential VMs |
| [`tdx`](tdx.md) | `.pmi.tdx` | Intel TDX confidential VMs (TODO)      |
| [`cca`](cca.md) | `.pmi.cca` | Arm CCA confidential VMs (TODO)        |

Targets are independent — they share conventions (the [`dtb`](dtb.md)
field; the [`load`](load.md) and [`dtbo`](dtbo.md) actions) but each one
fully specifies its own launch recipe. There is no inheritance, no
fallback, no selection logic beyond "the VMM targeting `sev` reads
`.pmi.sev`."

## Shape of a target spec

Every target spec is a CBOR map with the same outer shape:

```cddl
target = {
  "version" => uint,                ; schema version
  ? "dtb"   => tstr,                ; PE section containing the base DTB
  "actions" => [+ action],          ; ordered launch recipe
  * tstr => any,                    ; unknown keys ignored
}
```

Each target defines its own set of `action` types. Common action types
(used by multiple targets) are:

- [`load`](load.md) — load a PE section's bytes into guest memory.
- [`dtbo`](dtbo.md) — VMM writes a host-decided devicetree overlay into
  the named section.

Target-specific action types are defined by each target binding (e.g.,
`vcpu` on `vm`, `sev:policy` / `sev:id-block` / `sev:vmsa` / ... on `sev`).

Action `type` values use the `<target>:<name>` convention when scoped
(e.g., `sev:vmsa`); short, unscoped names (`load`, `dtbo`, `vcpu`) are used
where collisions are not a concern.

## VMM execution model

1. **Select target.** Identify the target and read its PE section (e.g.,
   `.pmi.sev` for SEV). If the section is absent, refuse to launch.
2. **Inspect DTB.** If the spec includes a [`dtb`](dtb.md), parse its FDT
   and validate that the host can satisfy every hardware capability it
   declares. Fail the launch if any declaration cannot be satisfied.
3. _(reserved)_
4. **Target initialize.** Initialize the target's cryptographic context,
   consuming any action whose type binds to this step (e.g., `sev:policy`).
5. _(reserved)_
6. **Process actions.** Process each action in array order. Each action's
   `type` selects how the VMM consumes it; common types load PE bytes into
   guest memory and are measured by the target's measurement API as
   appropriate.
7. _(reserved)_
8. **Target finalize.** Consume launch-finalize actions (e.g.,
   `sev:id-block` and `sev:id-auth`) and seal the measurement.
9. **Start the guest.**

Action order is security-critical on CC targets: the launch measurement is
an ordered hash chain, so reordering actions produces a different digest.

## Example: what a PMI image contains

A PMI image supporting both `vm` and SEV serviced boot might contain the
following PE sections. Only the `.pmi.<target>` names are used by PMI to
discover target specs; all other names shown are illustrative.

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
| `.pmi.vm`  | No              | `vm` target spec                           |
| `.pmi.sev` | No              | `sev` target spec                          |

On bare metal, UEFI executes the EFI stub, which boots the kernel from
`.linux`. All `.pmi.*` and other non-loaded PE sections are ignored.

A VMM targeting `vm` reads `.pmi.vm`, inspects its `dtb`, validates
conformance, processes its `actions` (load segments, write the overlay, set
the boot vCPU), and starts the guest.

A VMM targeting `sev` reads `.pmi.sev`. Its actions drive
`SNP_LAUNCH_START` (`sev:policy`, or policy embedded in the signed
`sev:id-block`), `SNP_LAUNCH_UPDATE` (`load` and
`sev:vmsa`/`sev:secrets`/`sev:cpuid`), and `SNP_LAUNCH_FINISH`
(`sev:id-block` + `sev:id-auth`), with the launch digest covering
everything fed to the target's measurement API.

## PE constraints and page granularity

PMI imposes alignment rules on PE sections that allow zero-copy loading
with 2M huge pages, and requires that target-spec sections be non-loaded.
See [PE constraints and page granularity](pe.md) for the full rules.
