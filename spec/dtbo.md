# `dtbo` action

The `dtbo` action carries a host-decided Devicetree Blob Overlay (FDT v17)
that extends the image's base [DTB](dtb.md) with runtime-decided
properties: vCPU enumeration, memory layout, and NUMA topology. See
[Overview](overview.md#solving-the-platform-definition-inversion) for the
conceptual role of the overlay.

This document is the normative reference for the overlay schema, content
constraints, and consumer-side validation rules. `dtbo` is a baseline
action type reused across multiple PMI targets; each target binding that
references `dtbo` is normative for how the action interacts with that
target's launch flow (measurement, page typing, ordering against other
actions). Where a target binding and this document conflict, the target
binding wins.

## Schema

```cddl
dtbo = {
  "type"    => "dtbo",
  "section" => tstr,                ; PE section to fill with the overlay
}
```

The referenced PE section MUST be a zero section (`SizeOfRawData == 0`,
`VirtualSize > 0`) — it reserves an address range with no on-disk data.
The VMM generates the overlay and writes it into the region. `dtbo`
actions are not measured: their content is VMM-generated and cannot be
predicted by a verifier.

The consumer that applies the overlay to the base DTB is not mandated by
this specification — a guest stub, the kernel itself when it supports
overlay-at-boot, or any other trusted in-guest agent. PMI defines only the
on-disk format of the overlay and the constraints on its content.

## Content whitelist

The overlay MUST contribute ONLY content that falls into one of the
following four categories. Any node or property outside this whitelist is
non-conformant; the consumer MUST reject the launch on any violation.

1. **Nodes and properties under `/cpus`** (CPU enumeration).
2. **Nodes and properties under `/memory@*`** (memory layout).
3. **Anything under `/distance-map`** (NUMA distance matrix).
4. **The `numa-node-id` property** added to any node the base DTB already
   declared (e.g., `/pci@*`, device nodes). This is the only property the
   host may add outside the first three paths; it may never appear with
   any other host-contributed property on the same node.

The overlay's `totalsize` MUST NOT exceed the PE section's `VirtualSize`.

## Consumer validation (normative)

The consumer MUST treat the overlay as adversarial input. The consumer
MUST reject the launch if any of the following validations fail.

**Structural.** The overlay MUST be a well-formed FDT (header magic
`0xd00dfeed`, version 17, all block offsets within `totalsize`, all
referenced strings null-terminated within the strings block, every
`FDT_BEGIN_NODE` paired with a corresponding `FDT_END_NODE`).

**Whitelist.** Every node and property the overlay touches MUST fall into
one of the four whitelist categories above.

**Address-bearing values.** For every host-contributed address (every
`/memory@*/reg` and any `/memory@*/linux,usable-memory` entry, plus every
`/cpus/cpu@N/cpu-release-addr` on architectures that use the spin-table
enable-method):

- All addresses MUST be within the architecture's canonical bounds
  (currently `< 2^48` for x86-64 and aarch64).
- No `address + size` computation MAY overflow.
- All declared regions MUST be pairwise non-overlapping with each other
  AND with each architecturally-fixed MMIO region declared in the image's
  base DTB (interrupt controllers, syscon devices, PCIe ECAM, etc.).
- The union of all `/memory@*/reg` regions MUST contain every loaded PE
  section's `[VirtualAddress, VirtualAddress + VirtualSize)` range.
- Each `cpu-release-addr` MUST lie inside a `/memory@*/reg` region AND
  MUST NOT overlap any loaded PE section's range.

**Bounded counts.** Implementations MUST enforce upper bounds on the
overlay byte length and the number of CPU nodes in the merged tree. The
recommended minimum upper bounds are 64 KiB and 256 CPU nodes
respectively; implementations MAY enforce smaller bounds where
resource-constrained.

**Phandle resolution.** Every phandle referenced by a host-contributed
property MUST resolve to a node present in the merged DTB.

## Non-validation

The consumer is NOT required to validate:

- The values of `numa-node-id` properties beyond structural type
  conformance (a kernel will tolerate or reject bad NUMA IDs through its
  own bounds; misassignment is at worst a denial-of-service).
- The values within `/distance-map/distance-matrix` (pure numeric hints
  for NUMA scheduling; bad values degrade performance but do not
  compromise the guest).
- The `compatible` strings on host-added CPU or device nodes (kernel-side
  driver curation is the appropriate defense against driver-specific
  attacks; this is out of scope for the dtbo consumer).
