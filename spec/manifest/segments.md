# Segments

The per-platform manifest's `segments` array is the launch recipe: an ordered
list of everything the VMM does during the platform's launch procedure. See
the [manifest schema](README.md#schema) for the top-level structure.

PMI uses ELF-style terminology: a **segment** is a runtime-loadable or
launch-procedure entry the VMM acts on, while a **PE section** is the
file-format container that supplies its bytes (or, for VMM-generated
segments, only its address range). Each segment references one PE section by
name.

There is no `platforms` filter on segments — the entire manifest is
platform-specific, selected by the [PMI index](../index.md). Every segment in
the manifest applies on that platform.

VMM-inspectable image data that is not loaded into guest memory (such as the
base [DTB](dtb.md) describing the image's expected platform topology) is
declared by the manifest's [`dtb`](dtb.md) field, not here.

## Schema

```cddl
segment = {
  "section"  => tstr,                  ; any PE section name (e.g., ".ovmf", ".sev.svm")
  ? "type"   => tstr,                  ; segment kind; default "pmi:load"
  * tstr => any,                       ; type-specific parameters
}
```

- **`section`** — the PE section this segment references. The value is the
  exact name of a PE section in the image; section names are free-form
  (subject only to PE's 8-byte limit). Conventional prefixes such as `.sev.`
  appear in examples but are not required. The VMM reads `VirtualAddress`,
  `SizeOfRawData`, `VirtualSize`, and `PointerToRawData` from this PE
  section's header.

- **`type`** — identifies the segment kind. Defaults to `"pmi:load"` when
  absent. See [Defined types](#defined-types) for the types this specification
  defines. Consumers MUST reject unknown type values; consumers MUST ignore
  unknown keys (including unknown type-specific parameters).

## Processing order

The VMM processes segments in array order. Each segment's `type` determines
which step of the platform's launch procedure consumes it; the platform
binding's execution-model table maps types to steps.

Within each step, segments of types bound to that step are processed in the
order they appear in the segments array. CC platforms measure pages in
submission order, so this ordering is security-critical: reordering segments
produces a different launch digest.

## Defined types

### `"pmi:load"` — Load PE section bytes

The default segment type. The VMM loads the referenced PE section's bytes into
guest memory at `VirtualAddress`, following the three PE patterns described in
[Segment loading](#segment-loading).

Type-specific parameters:

| Key          | Type   | Default | Meaning                                                  |
| ------------ | ------ | ------- | -------------------------------------------------------- |
| `"measured"` | `bool` | `true`  | Whether to feed bytes to the platform's measurement API. |

### `"pmi:dtbo"` — Devicetree Blob Overlay

The VMM MUST write a Devicetree Blob Overlay (FDT v17) into the segment at
`VirtualAddress`, conveying the host-decided runtime properties that extend the
image's base [DTB](dtb.md): vCPU enumeration, memory layout, and NUMA topology.

The referenced PE section MUST be a zero section (`SizeOfRawData == 0`,
`VirtualSize > 0`) — it reserves an address range with no on-disk data. The VMM
generates the overlay and writes it into the region. Segments of this type are
not measured: their content is VMM-generated and cannot be predicted by a
verifier.

The overlay is applied to the base DTB by a consumer inside the guest. This
specification does not mandate the consumer's identity (a guest stub, the kernel
itself when it supports overlay-at-boot, or any other trusted in-guest agent).
PMI defines only the on-disk format of the overlay and the constraints on its
content.

#### Content whitelist

The overlay MUST contribute ONLY content that falls into one of the following
four categories. Any node or property outside this whitelist is non-conformant;
the consumer MUST reject the launch on any violation.

1. **Nodes and properties under `/cpus`** (CPU enumeration).
2. **Nodes and properties under `/memory@*`** (memory layout).
3. **Anything under `/distance-map`** (NUMA distance matrix).
4. **The `numa-node-id` property** added to any node the base DTB already
   declared (e.g., `/pci@*`, device nodes). This is the only property the host
   may add outside the first three paths; it may never appear with any other
   host-contributed property on the same node.

The overlay's `totalsize` MUST NOT exceed the segment's `VirtualSize`.

#### Consumer validation (normative)

The consumer MUST treat the overlay as adversarial input. The consumer MUST
reject the launch if any of the following validations fail.

**Structural.** The overlay MUST be a well-formed FDT (header magic
`0xd00dfeed`, version 17, all block offsets within `totalsize`, all referenced
strings null-terminated within the strings block, every `FDT_BEGIN_NODE` paired
with a corresponding `FDT_END_NODE`).

**Whitelist.** Every node and property the overlay touches MUST fall into one of
the four whitelist categories above.

**Address-bearing values.** For every host-contributed address (every
`/memory@*/reg` and any `/memory@*/linux,usable-memory` entry, plus every
`/cpus/cpu@N/cpu-release-addr` on architectures that use the spin-table
enable-method):

- All addresses MUST be within the architecture's canonical bounds (currently
  `< 2^48` for x86-64 and aarch64).
- No `address + size` computation MAY overflow.
- All declared regions MUST be pairwise non-overlapping with each other AND with
  each architecturally-fixed MMIO region declared in the image's base DTB
  (interrupt controllers, syscon devices, PCIe ECAM, etc.).
- The union of all `/memory@*/reg` regions MUST contain every PMI segment's
  `[VirtualAddress, VirtualAddress + VirtualSize)` range.
- Each `cpu-release-addr` MUST lie inside a `/memory@*/reg` region AND MUST NOT
  overlap any PMI segment's range.

**Bounded counts.** Implementations MUST enforce upper bounds on the overlay
byte length and the number of CPU nodes in the merged tree. The recommended
minimum upper bounds are 64 KiB and 256 CPU nodes respectively; implementations
MAY enforce smaller bounds where resource-constrained.

**Phandle resolution.** Every phandle referenced by a host-contributed property
MUST resolve to a node present in the merged DTB.

#### Non-validation

The consumer is NOT required to validate:

- The values of `numa-node-id` properties beyond structural type conformance (a
  kernel will tolerate or reject bad NUMA IDs through its own bounds;
  misassignment is at worst a denial-of-service).
- The values within `/distance-map/distance-matrix` (pure numeric hints for NUMA
  scheduling; bad values degrade performance but do not compromise the guest).
- The `compatible` strings on host-added CPU or device nodes (kernel- side
  driver curation is the appropriate defense against driver- specific attacks;
  this is out of scope for the DTBO consumer).

### Platform-defined types

Each platform binding may define additional segment types under its own
namespace. These types describe how segments are handed to platform-specific
launch APIs and at which launch step they are consumed:

- [AMD SEV 3.0](platforms/sev.md) — page-load types (`pmi:sev:vmsa`,
  `pmi:sev:secrets`, `pmi:sev:cpuid`) plus launch-input types
  (`pmi:sev:policy`, `pmi:sev:id-block`, `pmi:sev:id-auth`)
- [VM](platforms/vm.md) — `pmi:vm:vcpu`

## Segment loading

For each `pmi:load` segment, the VMM reads the referenced PE section header
and determines how to load it based on `VirtualAddress`, `SizeOfRawData`,
`VirtualSize`, and `PointerToRawData`.

The VMM loads pages from the lowest GPA to the highest within each segment.
This ordering is significant: CC platforms measure pages in submission order, so
lowest-to-highest produces a deterministic measurement.

There are three PE-section shapes:

1. **Data** (`SizeOfRawData > 0`, `VirtualSize == SizeOfRawData`). Load the
   on-disk data at `VirtualAddress`. The VMM chooses page granularity based on
   alignment — see [overview](../pe.md#page-granularity).

2. **Padded** (`SizeOfRawData > 0`, `VirtualSize > SizeOfRawData`). Load the
   on-disk data at `VirtualAddress` as in case 1. Then zero-fill from
   `VirtualAddress + SizeOfRawData` to `VirtualAddress + VirtualSize`. The
   trailing zero region SHOULD use the platform's zero-page API where available
   (e.g., `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_ZERO`), which validates pages as
   zero without transferring data. This is standard PE .bss-tail behavior —
   firmware or service modules that need reserved memory beyond their code use
   this to express it without file backing.

3. **Zero** (`SizeOfRawData == 0`, `VirtualSize > 0`). The entire region is
   zero-filled. No disk data is loaded. The VMM SHOULD use the platform's
   zero-page API for the full range. This is also the shape required by
   `pmi:dtbo` and by platform-defined types that reserve address space for
   VMM-generated content.

## Measurement

Measurement behavior is determined by the segment's type:

- **`pmi:load`** is measured by default. Setting `"measured": false` suppresses
  measurement (e.g., for VMM-supplied data the verifier does not need to bind).
- **`pmi:dtbo`** is never measured.
- **Platform-defined types** specify their measurement semantics in the
  platform binding — typically the platform's measurement API binds the GPA
  and page type without binding content for VMM-populated pages, and
  launch-input types are not measured at all (they feed the platform's
  init/finalize APIs rather than the per-page measurement chain).

The distinction between on-disk data and zero-fill matters for `pmi:load`
measurement. On-disk bytes are measured as normal data pages. Zero-filled bytes
are measured as zero pages using the platform's zero-page measurement semantic,
which may produce a different measurement than loading actual zeros as data
pages. VMM implementations MUST NOT substitute data-page loads for zero-page
operations or vice versa.

In serviced configurations, the launch measurement covers the service module and
firmware. Kernel boot is measured separately by firmware via the service
module's virtual TPM (vTPM) into runtime measurement registers. A verifier needs
both the launch digest and the runtime measurement quotes — neither alone is
sufficient.
