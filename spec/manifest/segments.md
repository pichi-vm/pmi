# Segments

The manifest's `segments` array is the core of a PMI image. It is an ordered
list of everything the VMM loads into guest memory or generates when launching a
guest. See the [manifest schema](README.md#schema) for the top-level structure.

PMI uses ELF-style terminology: a **segment** is a runtime-loadable entry the
VMM acts on, while a **PE section** is the file-format container that supplies
its bytes (or, for filled segments, only its address range). Each segment
references one PE section by name.

VMM-inspectable image data that is not loaded into guest memory (such as the
base [DTB](dtb.md) describing the image's expected platform topology) is
declared in the [metadata](metadata.md) array, not here.

## Schema

```cddl
segment = {
  "name"         => tstr,               ; PE section name (e.g., ".ovmf", ".sev.svm")
  ? "fill"        => fill,              ; VMM-generated content; absent = load from disk
  ? "platforms"   => { + tstr => any },  ; platform name => platform-defined annotation
  ? "measured"    => bool,              ; default true
  * tstr => any,                        ; extension point
}
```

## Extensibility

Every PMI-defined map accepts additional keys beyond those defined here.
Well-known keys are short, unnamespaced strings (e.g., `"name"`, `"measured"`,
`"sev"`). Extension keys MUST use a collision-resistant namespaced form:
`"namespace:key"` (e.g., `"vendor:feature"`). Well-known fill values use the
same namespaced convention.

Consumers MUST ignore keys and fill values they do not recognize.

## Processing Order

The VMM processes segments in array order during
[step 6](../overview.md#vmm-execution-model) of the execution model. Measurement
follows the same order.

Each segment references a PE section by name. The VMM reads `VirtualAddress`,
`SizeOfRawData`, `VirtualSize`, and `PointerToRawData` from the PE section
header.

## Data Segments

When `fill` is absent, the segment is a data segment. The VMM loads on-disk data
from the PE into guest memory at `VirtualAddress`.

## Filled Segments

When `fill` is present, the referenced PE section has `SizeOfRawData == 0` and
`VirtualSize > 0` — it reserves an address range with no on-disk data. The VMM
generates content based on the `fill` value and writes it into the region at
`VirtualAddress`. Filled segments SHOULD be unmeasured (`"measured": false`)
since their content is VMM-generated and cannot be predicted by a verifier.

A segment MUST NOT have both `fill` and a non-null platform annotation.

The `fill` field is a map with a required `"type"` key that identifies the fill
type. Additional keys are type-specific parameters.

```cddl
fill = {
  "type"         => tstr,              ; fill type identifier
  * tstr => any,                       ; type-specific parameters
}
```

VMMs MUST reject fill types they do not recognize. Well-known fill types use the
`"namespace:type"` convention. The following are defined by PMI:

### `"pmi:dtbo"` — Devicetree Blob Overlay

The VMM MUST write a Devicetree Blob Overlay (FDT v17) into the segment at
`VirtualAddress`, conveying the host-decided runtime properties that extend the
image's base [DTB](dtb.md): vCPU enumeration, memory layout, and NUMA topology.

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

## Platform Annotations

The `platforms` field, when present, is a map from platform name to a
platform-defined value. If the current platform is not a key in the map, the
segment is skipped.

- A `null` value means "load this segment on this platform with no special
  behavior."
- A non-null value is interpreted by the platform adapter — for example, SEV 3.0
  uses string values to indicate page types (`"vmsa"`, `"secrets"`, `"cpuid"`).
  See [platforms/sev.md](platforms/sev.md) for details.

Segments with a non-null platform annotation are loaded in step 6 using the
platform adapter's segment-specific API, in array order alongside all other
segments.

If `platforms` is absent, the segment is loaded on all platforms during step 6.

## Segment Loading

For each segment loaded in step 6, the VMM reads the referenced PE section
header and determines how to load it based on `VirtualAddress`, `SizeOfRawData`,
`VirtualSize`, and `PointerToRawData`.

The VMM loads pages from the lowest GPA to the highest within each segment.
This ordering is significant: CC platforms measure pages in submission order, so
lowest-to-highest produces a deterministic measurement.

There are three cases:

1. **Data segment** (`SizeOfRawData > 0`, `VirtualSize == SizeOfRawData`). Load
   the on-disk data at `VirtualAddress`. The VMM chooses page granularity based
   on alignment — see [overview](../pe.md#page-granularity).

2. **Padded segment** (`SizeOfRawData > 0`, `VirtualSize > SizeOfRawData`). Load
   the on-disk data at `VirtualAddress` as in case 1. Then zero-fill from
   `VirtualAddress + SizeOfRawData` to `VirtualAddress + VirtualSize`. The
   trailing zero region SHOULD use the platform's zero-page API where available
   (e.g., `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_ZERO`), which validates pages as
   zero without transferring data. This is standard PE .bss-tail behavior —
   firmware or service modules that need reserved memory beyond their code use
   this to express it without file backing.

3. **Zero segment** (`SizeOfRawData == 0`, `VirtualSize > 0`). The entire region
   is zero-filled. No disk data is loaded. The VMM SHOULD use the platform's
   zero-page API for the full range. This is how reserved memory regions are
   expressed — for example, SEV secrets pages and CPUID pages that the platform
   adapter populates via their platform annotation.

## Measurement

If `measured` is true (the default), the segment's data is fed to the platform's
measurement API during loading.

The distinction between on-disk data and zero-fill matters for measurement.
On-disk bytes are measured as normal data pages. Zero-filled bytes are measured
as zero pages using the platform's zero-page measurement semantic, which may
produce a different measurement than loading actual zeros as data pages. VMM
implementations MUST NOT substitute data-page loads for zero-page operations or
vice versa.

Filled segments SHOULD be unmeasured since their content is VMM-generated.
Platform-annotated segments are measured by the platform as appropriate — the
measurement rules for platform-annotated segments are defined by the platform's
binding specification, not by PMI.

In serviced configurations, the launch measurement covers the service module and
firmware. Kernel boot is measured separately by firmware via the service
module's virtual TPM (vTPM) into runtime measurement registers. A verifier needs
both the launch digest and the runtime measurement quotes — neither alone is
sufficient.
