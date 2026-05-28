# `merged` Extension

**Prefix:** `merged`.

The `merged` extension splits the guest's platform description into two layers
with different trust models. The image carries a base flattened devicetree blob
(DTB) describing only platform definition; the host supplies a flattened
devicetree overlay (DTBO) restricted to a resource-allocation allowlist; the
guest merges the overlay onto the base and validates the result. The base DTB is
loaded as a measured action where the target measures loads; the overlay is
unmeasured.

It defines two extension points:

1. The new target attribute [`merged:dtb`](#1-new-target-attribute-mergeddtb).
2. The new `fill` kind [`merged:dtbo`](#2-new-fill-kind-mergeddtbo).

See
[Motivation §2](motivation.md#splitting-platform-definition-from-resource-allocation)
for the trust model.

## 1. New target attribute: `merged:dtb`

The `merged:dtb` target attribute names the PE section holding the base DTB:

```cddl
merged-dtb = tstr                    ; PE section name
```

A VMM MUST refuse to launch on any of:

- the active target's `actions` contain a `merged:dtbo` fill but the target spec
  lacks a `merged:dtb` attribute, or vice versa;
- the section named by `merged:dtb` is not a PE section present in the image;
- the bytes at the named section do not parse as a well-formed flattened
  devicetree blob in the format defined by the [Devicetree
  Specification][devicetree] v0.4 or later;
- the base DTB lacks a `/cpus/cpu@0` node providing the properties required of
  every CPU the guest will run (`device_type`, `compatible`, `enable-method`,
  and any other architecturally-required properties).

The VMM reads the base DTB from the PE file to inform overlay generation. The
image author chooses whether to also place it in guest memory via a `load`
action; if loaded, the bytes contribute to the launch measurement on
confidential targets like any other load. If not loaded, the in-guest consumer
obtains its measured copy of the base DTB by other means (typically by embedding
it in the measured consumer binary).

The base DTB carries only platform definition — MMIO map, interrupt controller,
transport choice, device topology — and the `cpu@0` template the host's overlay
extends (see [§2 Allowlist](#allowlist)). Resource-allocation surfaces
(additional `cpu@N` nodes, `/memory@*`, `/distance-map`, per-node
`numa-node-id`) come from the overlay in [§2](#2-new-fill-kind-mergeddtbo).

## 2. New `fill` kind: `merged:dtbo`

The VMM fills the section with a host-supplied flattened devicetree overlay
(DTBO), in the format defined by the [Devicetree Specification][devicetree] v0.4
or later. The host selects the overlay content via VMM-defined input, out of
scope for PMI. The overlay is **unmeasured** — it does not contribute to the
target's launch measurement.

The VMM MUST construct the overlay such that:

- it is a well-formed FDT v17 blob following the overlay convention (a sequence
  of top-level `fragment@N` nodes, each carrying `target` or `target-path` and
  an `__overlay__` subnode);
- its byte length is bounded (recommended ≤ 64 KiB);
- every host-contributed address (every `/memory@*/reg` and any
  `/memory@*/linux,usable-memory` entry) lies within the architecture's
  canonical bounds (currently `< 2^48` for x86-64 and aarch64);
- host-contributed `/memory@*` regions do not overlap each other;
- every host-contributed phandle resolves to a node present in the merged DTB;
- the merged DTB's usable RAM — the union of `reg` entries on nodes with
  `device_type = "memory"` whose `status` is absent or `"okay"`, minus any range
  covered by a `/reserved-memory` child carrying the `no-map` property — covers
  every load/fill range in the active target's `actions`, including the sections
  holding the base DTB and the overlay.

Violations of these requirements manifest as kernel boot failures (DoS) and are
not consumer-validated.

The overlay is adversarial input from the host. The guest MUST parse the
overlay, validate it against the rules in [Validation](#validation), and merge
it onto the base DTB named by `merged:dtb` before relying on the platform
description. The merger's implementation is out of scope for this spec.

### Validation

The merger MUST reject the launch on any of the following.

**Allowlist.** Every node and property the overlay contributes MUST fall into
one of the following four categories. Anything outside is non-conformant.

1. Under `/cpus`: the overlay MAY add `cpu@N` nodes for any `N` not already
   in the base DTB. The overlay MUST NOT modify properties on `cpu@N` nodes
   already present in the base DTB. The overlay MUST NOT set `phandle` or
   `linux,phandle` on any added `cpu@N`.

   Each added `cpu@N` MUST carry `reg` and MAY carry `numa-node-id`. Apart
   from these two properties, the added node's property set MUST be exactly
   equal to `cpu@0`'s, with byte-identical values: neither addition (a
   property absent from `cpu@0`) nor omission (a `cpu@0` property the
   overlay does not copy) is permitted. `phandle` and `linux,phandle` are
   excluded from this comparison — `cpu@0` MAY carry them (the merger
   renumbers them at apply time) but the overlay MUST NOT, per the
   prohibition above.

2. Nodes and properties under `/memory@*`.
3. Nodes and properties under `/distance-map`.
4. The `numa-node-id` property added to any node the base DTB already declared.
   This is the only property the host MAY add outside the first three paths, and
   it MUST NOT appear with any other host-contributed property on the same node.

**Address-bearing values.** For every host-contributed address:

- Every `address + size` computation MUST NOT overflow the architecture's
  address width.
- All declared `/memory@*` regions MUST be pairwise non-overlapping with every
  base-DTB node bearing a `reg` property.

### Non-validation

The merger is NOT required to validate:

- The values of `numa-node-id` properties beyond structural type conformance.
- The values within the `distance-matrix` property under `/distance-map`.

[devicetree]: https://www.devicetree.org/specifications/

## Example

A `.pmi.vm` that loads a kernel, initrd, command line, and base DTB, and fills
`.dtbo` with a host-supplied overlay:

```cbor-diag
{
  "version": 1,
  "vm:vcpu": {"rip": 0x100000, "rsp": 0x80000, "rflags": 0x2},
  "merged:dtb": ".dtb",
  "actions": [
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"},
    {"type": "load", "section": ".dtb"},
    {"type": "fill", "section": ".dtbo", "kind": "merged:dtbo"}
  ]
}
```
