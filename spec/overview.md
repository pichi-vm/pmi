# Overview

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

This document explains how PMI addresses the two problems framed in
[Motivation](motivation.md) and introduces the file structure and launch
model that express the solution. See [Examples](examples.md) for concrete
walkthroughs.

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
they carry data the loader does not need at runtime. PMI imposes alignment
rules on PE sections that allow zero-copy loading with 2M huge pages, and
requires that target-spec sections be non-loaded; see
[PE constraints and page granularity](pe.md) for the full rules.

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

## Launching a VM with PMI

A VMM consumes exactly one target spec per launch: the CBOR map in the
image's `.pmi.<target>` section for the target it is launching. Target
specs share a similar outer shape — a schema version, a reference to a
base DTB, and an ordered `actions` array — but each target binding is
normative for its own schema.

The VMM executes the launch in the following ordered steps:

1. **Select target.** Read the `.pmi.<target>` PE section. Refuse to
   launch if it is absent.
2. **Inspect DTB.** Parse the FDT named by the spec's [`dtb`](dtb.md)
   field and validate that the host can satisfy every hardware capability
   it declares. Fail the launch if any declaration cannot be satisfied.
3. **Target initialize.** Perform target-defined initialization (for
   example, establishing a cryptographic launch context). The inputs and
   criteria for this phase are defined by the target binding.
4. **Process actions.** Process each entry in the `actions` array in
   order. Each action's `type` field selects how the VMM consumes it;
   the target's measurement API (where applicable) is fed in the same
   order.
5. **Target finalize.** Perform target-defined finalization (for example,
   sealing the launch measurement). The inputs and criteria for this
   phase are defined by the target binding.
6. **Start the guest.**

Action order is security-critical on confidential targets: the launch
measurement is an ordered hash chain, so reordering actions produces a
different digest.

The runtime overlay described under
[platform-definition inversion](#solving-the-platform-definition-inversion)
reaches the guest through a [`dtbo`](dtbo.md) action processed during
step 4. A `dtbo` action names a zero PE section — a section that
reserves a GPA range but carries no on-disk data. The VMM generates the
overlay for this launch (the `/cpus`, `/memory@*`, and `/distance-map`
subtrees, plus any `numa-node-id` annotations on image-declared nodes)
and writes it into the reserved range. The overlay is generated fresh
per launch and is not measured; the guest is responsible for merging it
onto the base DTB before booting.

The other action types each target accepts are defined in the target's
binding.

