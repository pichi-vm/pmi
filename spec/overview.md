# Overview

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

This document explains how PMI addresses the two problems framed in
[Motivation](motivation.md), then introduces the file structure that
expresses the solution.

## Solving the platform-definition inversion

PMI inverts the host-defines-platform pattern by making the image
declarative about what hardware platform it expects.

The image carries a base **Devicetree Blob (DTB)** describing its expected
platform: virtual devices and their MMIO ranges, the interrupt controller,
the PCIe topology, the console, reserved-memory regions, the `/chosen`
parameters — everything outside the runtime-decided subtrees (`/cpus`,
`/memory@*`, `/distance-map`). The VMM reads this DTB before launching the
guest and is obligated to provide exactly what the DTB declares — every
device at the declared GPA, every interrupt controller version, every PCIe
layout. Anything the host cannot match is grounds for refusing to launch.
The host has effectively infinite flexibility (it is software) to configure
itself to match; the responsibility for matching is on the host.

The host's runtime decisions — how much memory this particular guest gets,
how many vCPUs, how those vCPUs and memory are arranged across NUMA nodes —
cannot be known to the image author in advance. The VMM supplies these
through a **Devicetree Blob Overlay (DTBO)** it writes into a known segment
at launch. The overlay is restricted by a content whitelist to exactly the
three subtrees `/cpus`, `/memory@*`, and `/distance-map`, plus a single
property (`numa-node-id`) that may be added to image-declared nodes.
Anything outside the whitelist is rejected by the guest before the overlay
is applied.

The result: the platform the guest boots against is the platform the image
declared, plus a sharply bounded runtime delta that the guest validates
against an explicit rule set. The validation surface is small enough to
reason about, and the bulk of the platform (the base DTB) is bound into the
launch measurement.

See [dtb.md](dtb.md) for the base DTB format, conformance contract, and
image-side responsibilities; see [dtbo.md](dtbo.md) for the overlay schema,
content whitelist, and consumer validation rules.

## Solving the single-artifact problem

PMI inherits PE so a single binary can serve every shape of Linux boot,
from bare metal to confidential VM, without per-shape image variants.

### PE and UKI as background

PE (Portable Executable) is the binary format that UEFI firmware loads and
executes. A PE file is divided into named **sections**, each with a header
describing where its bytes live in the file and where they should be mapped
into memory. Sections marked `IMAGE_SCN_MEM_DISCARDABLE` are not mapped —
they carry data the loader does not need at runtime. See
[PE constraints and page granularity](pe.md) for the fields PMI reads.

A Unified Kernel Image (UKI) is a PE file that bundles a kernel (`.linux`),
an initial ramdisk (`.initrd`), a command line (`.cmdline`), and an EFI stub
into named PE sections. UEFI executes the stub; the stub loads the kernel
and boots it. PMI builds on this same PE-with-named-sections idiom.

### PMI as a PE extension

A PMI image is a PE binary. It MAY also be structured as a UKI (carrying
`.linux`, `.initrd`, `.cmdline`, and an EFI stub) for bare-metal and
stubbed VM paths; UEFI ignores the PMI-specific sections. A PMI image is
not _required_ to be UKI-shaped — an image that contains only firmware
(for OVMF-loads-kernel-from-disk modes), or only confidential-VM content,
is equally valid. PMI is compatible with UKI, not a flavor of it.

PMI's extension to PE is a set of non-loaded sections whose names begin
with `.pmi.` — one per launch target the image supports.

### Targets

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

Targets are independent — they share conventions but each one fully
specifies its own launch recipe. There is no inheritance, no fallback, no
selection logic beyond "the VMM targeting `sev` reads `.pmi.sev`."

### Shape of a target spec

Every target spec is a CBOR map with the same outer shape:

```cddl
target = {
  "version" => uint,                ; schema version
  ? "dtb"   => tstr,                ; PE section containing the base DTB
  "actions" => [+ action],          ; ordered launch recipe
  * tstr => any,                    ; unknown keys ignored
}
```

Each target defines its own set of `action` types. The reference docs
[`load`](load.md) and [`dtbo`](dtbo.md) describe action types that recur
across multiple targets — they specify a baseline schema and a default
semantic, but the target binding (e.g., [`vm`](vm.md), [`sev`](sev.md)) is
normative for what the action does within that target. Other action types
are defined by a single target (e.g., `vcpu` on `vm`, `sev:policy` /
`sev:id-block` / `sev:vmsa` / ... on `sev`).

Action `type` values use the `<target>:<name>` convention when scoped
(e.g., `sev:vmsa`); short, unscoped names (`load`, `dtbo`, `vcpu`) are used
where collisions are not a concern.

### VMM execution model

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

### Example: what a PMI image contains

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
