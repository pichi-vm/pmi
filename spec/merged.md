# `merged` Extension

**Prefix:** `merged`.

The `merged` extension splits the guest's platform description into two layers
with different trust models. The image carries a base flattened devicetree blob
(DTB) describing only platform definition; the host supplies a flattened
devicetree overlay (DTBO) restricted to an allowlist of facts the host alone
knows at launch — resource allocation, plus the CPUs' implementation identity;
the guest merges the overlay onto the base and validates the result. The base DTB is
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
- the base DTB lacks a `/cpus/cpu@0` node, or that `cpu@0` lacks the
  build-time-known properties required of every CPU the guest will run:
  `device_type`, `reg`, and any architecturally-required bring-up property
  (`enable-method` on aarch64);
- any CPU node in the base DTB carries a `compatible` property. A CPU's identity
  is a launch-time fact the host supplies via the overlay (see
  [§2 Allowlist](#allowlist)); the image does not know which core it will run on
  (the [`cpu:profile`](cpu.md) it declares is a *floor*, not the actual
  implementation), so a base DTB that hard-codes a CPU `compatible` is
  non-conformant.

The VMM reads the base DTB from the PE file to inform overlay generation. The
image author chooses whether to also place it in guest memory via a `load`
action; if loaded, the bytes contribute to the launch measurement on
confidential targets like any other load. If not loaded, the in-guest consumer
obtains its measured copy of the base DTB by other means (typically by embedding
it in the measured consumer binary).

The base DTB carries only platform definition the image author knows at build
time — MMIO map, interrupt controller, transport choice, device topology — and
the `cpu@0` template (`device_type`, `reg`, `status`, and the arch bring-up
method) the host's overlay extends (see [§2 Allowlist](#allowlist)). Host-known
surfaces — additional `cpu@N` nodes, the CPUs' `compatible`, `/memory@*`,
`/distance-map`, and per-node `numa-node-id` — come from the overlay in
[§2](#2-new-fill-kind-mergeddtbo).

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
one of the following five categories. Anything outside is non-conformant.

1. Under `/cpus`: the overlay MAY add `cpu@N` nodes for any `N` not already
   in the base DTB. The overlay MUST NOT modify properties on `cpu@N` nodes
   already present in the base DTB. The overlay MUST NOT set `phandle` or
   `linux,phandle` on any added `cpu@N`.

   Each added `cpu@N` MUST carry `reg` and MAY carry `numa-node-id` and
   `compatible` (the latter governed by category 5). Apart from these three
   properties, the added node's property set MUST be exactly
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
   it MUST NOT appear with any other host-contributed property on the same node,
   except the CPU `compatible` of category 5 on `cpu@0`.

5. The `compatible` property on CPU nodes — the CPUs' implementation identity, a
   launch-time fact the host knows and the image does not. The overlay MAY
   contribute a single CPU `compatible` value. When it does:

   - it MUST be added to `/cpus/cpu@0` (overriding, for `compatible` only, the
     category-1 prohibition on modifying base CPU nodes) and to every
     overlay-added `cpu@N`;
   - it MUST be byte-identical on every CPU node — the guest's CPUs are
     homogeneous; per-CPU divergence (heterogeneous topology) is out of scope
     for this extension;
   - it is excluded from the `cpu@N`↔`cpu@0` template comparison of category 1,
     exactly as `reg` and `numa-node-id` are.

   The overlay MUST NOT contribute any other CPU-identity or CPU-feature
   property (`enable-method`, `capacity-dmips-mhz`, `clocks`, cache or topology
   nodes); those are the image's, or out of scope. Whether to contribute a CPU
   `compatible` at all is the VMM's choice: an architecture with a registered
   CPU `compatible` vocabulary (e.g. aarch64) SHOULD receive one; an
   architecture without (e.g. x86-64, which has no CPU `compatible` binding)
   receives none, and its CPU nodes carry only `device_type` + `reg`, which is
   conformant.

**The CPU `compatible` is non-authoritative.** It is host-supplied, unmeasured,
and on confidential targets adversarial. Guests and remote verifiers MUST derive
actual CPU identity and features from the architectural identification registers
(`MIDR_EL1` on aarch64, `CPUID` on x86-64) and, on attested targets, the
target's attestation report — never from this property. The allowlist admits
only the `compatible` string itself, not the `clocks` / `operating-points` /
topology nodes a driver match would require, and CPU errata are keyed on
identification registers rather than `compatible`; the value is therefore inert
and cannot alter guest behavior.

**Address-bearing values.** For every host-contributed address:

- Every `address + size` computation MUST NOT overflow the architecture's
  address width.
- All declared `/memory@*` regions MUST be pairwise non-overlapping with every
  base-DTB node bearing a `reg` property.

### Non-validation

The merger is NOT required to validate:

- The values of `numa-node-id` properties beyond structural type conformance.
- The values within the `distance-matrix` property under `/distance-map`.
- The value of the CPU `compatible` beyond structural type conformance and the
  cross-CPU byte-identity check; the guest cannot know the host's actual core to
  compare against, and per the clause above must not rely on it.

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
