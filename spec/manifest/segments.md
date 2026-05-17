# Segments

The manifest's `segments` array is the core of a PMI image. It is an ordered
list of everything the VMM loads into guest memory or generates when launching a
guest. See the [manifest schema](README.md#schema) for the top-level structure.

PMI uses ELF-style terminology: a **segment** is a runtime-loadable entry the
VMM acts on, while a **PE section** is the file-format container that supplies
its bytes (or, for VMM-generated segments, only its address range). Each segment
references one PE section by name.

VMM-inspectable image data that is not loaded into guest memory (such as the
base [DTB](dtb.md) describing the image's expected platform topology) is
declared in the [info](info.md) array, not here.

## Schema

```cddl
segment = {
  ? "platforms"  => { + tstr => any },  ; platform filter; absent = all platforms
  "section"      => tstr,                ; PE section name (e.g., ".ovmf", ".sev.svm")
  ? "type"        => tstr,               ; segment kind; default "pmi:load"
  * tstr => any,                        ; type-specific parameters
}
```

- **`platforms`** — restricts the segment to the listed platforms. If present
  and the current platform is not a key in the map, the segment is skipped. If
  absent, the segment applies on every platform. The map's values are reserved
  for future per-platform extensions; current PMI-defined types ignore them and
  use `null` in examples.

- **`section`** — the PE section this segment references. The VMM reads
  `VirtualAddress`, `SizeOfRawData`, `VirtualSize`, and `PointerToRawData` from
  this PE section's header.

- **`type`** — identifies the segment kind. Defaults to `"pmi:load"` when
  absent. See [Defined types](#defined-types) for the types this specification
  defines and [Extensibility](#extensibility) for the namespacing rules.

## Extensibility

Every PMI-defined map accepts additional keys beyond those defined here.
Well-known keys are short, unnamespaced strings (e.g., `"section"`, `"type"`,
`"platforms"`). Extension keys MUST use a collision-resistant namespaced form:
`"namespace:key"` (e.g., `"vendor:feature"`).

Type values defined by this specification use the `"pmi:"` prefix (e.g.,
`"pmi:load"`, `"pmi:dtbo"`, `"pmi:sev:vmsa"`). Extension type values MUST use a
namespaced form with a non-`"pmi:"` prefix (e.g., `"vendor:custom"`). VMMs MUST
reject type values they do not recognize.

Consumers MUST ignore keys and type-specific parameters they do not recognize.

## Processing Order

The VMM processes segments in array order during
[step 6](../overview.md#vmm-execution-model) of the execution model. Measurement
follows the same order.

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
APIs and what measurement semantics apply. See:

- [AMD SEV 3.0](platforms/sev.md) — `pmi:sev:vmsa`, `pmi:sev:secrets`,
  `pmi:sev:cpuid`
- [Native](platforms/native.md) — `pmi:native:vcpu`

A platform-defined type only makes sense when the segment is gated to that
platform via `platforms`. The platform binding specifies the required filter.

## Segment loading

For each `pmi:load` segment processed in step 6, the VMM reads the referenced PE
section header and determines how to load it based on `VirtualAddress`,
`SizeOfRawData`, `VirtualSize`, and `PointerToRawData`.

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
  and page type without binding content for VMM-populated pages.

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
