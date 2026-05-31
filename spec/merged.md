# `merged` Extension

**Prefix:** `merged`.

The `merged` extension splits the guest's platform description into two layers
with different trust models. The image carries a base flattened devicetree blob
(DTB) describing only platform definition; the host supplies a flattened
devicetree overlay (DTBO) restricted to a resource-allocation allowlist —
memory, and the CPU instances themselves;
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
- the base DTB lacks a `/cpus` container node, or that `/cpus` contains any
  `cpu@N` child. CPUs are host-provided resources: the base declares the
  `/cpus` container (with `#address-cells`/`#size-cells`), but the CPU
  *instances* — their count and per-CPU `reg`, `status`, `enable-method`, and
  `compatible` — are authored entirely by the host overlay (see
  [§2 Allowlist](#allowlist)).

The VMM reads the base DTB from the PE file to inform overlay generation. The
image author chooses whether to also place it in guest memory via a `load`
action; if loaded, the bytes contribute to the launch measurement on
confidential targets like any other load. If not loaded, the in-guest consumer
obtains its measured copy of the base DTB by other means (typically by embedding
it in the measured consumer binary).

The base DTB carries only platform definition the image author knows at build
time — MMIO map, interrupt controller, transport choice, device topology, and
the empty `/cpus` container. The CPU instances and resource-allocation surfaces
— every `cpu@N` node (with its `reg`, `status`, `enable-method`, and
`compatible`), `/memory@*`, `/distance-map`, and per-node `numa-node-id` — come
from the overlay in [§2](#2-new-fill-kind-mergeddtbo).

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

1. Under `/cpus`: the overlay authors the CPU instances. It MAY add `cpu@N`
   nodes for any `N` (the base contains none). Each `cpu@N`'s properties —
   `device_type` (= `"cpu"`), `reg`, and any of `status`, `enable-method`,
   `compatible` — are host-authored; the overlay MUST NOT set `phandle` or
   `linux,phandle`. The total CPU count MUST be bounded (recommended ≤ a
   VMM-defined maximum) to prevent resource exhaustion.

   The CPUs are homogeneous in identity and bringup: the overlay SHOULD give
   every `cpu@N` the same `compatible` and `enable-method`. `status` is
   per-CPU — the boot CPU MUST be `okay`, while others MAY be `disabled`
   (e.g. offline-capable / hot-onlineable) — and `reg` MUST be unique.
   Malformed or inconsistent CPU nodes (a missing or duplicate `reg`, a
   non-`okay` boot CPU, an `enable-method` the platform does not implement)
   manifest as boot failures (DoS) and are not consumer-validated. Per-CPU
   divergence in identity (heterogeneous topology) is out of scope for this
   extension.

2. Nodes and properties under `/memory@*`.
3. Nodes and properties under `/distance-map`.
4. The `numa-node-id` property added to any node the base DTB already declared.
   This is the only property the host MAY add outside the first three paths, and
   it MUST NOT appear with any other host-contributed property on the same node.

**The CPU `compatible` is non-authoritative.** It is host-supplied, unmeasured,
and on confidential targets adversarial. Guests and remote verifiers MUST derive
actual CPU identity and features from the architectural identification registers
(`MIDR_EL1` on aarch64, `CPUID` on x86-64) and, on attested targets, the
target's attestation report — never from this property. CPU errata are keyed on
identification registers rather than `compatible`, so the value is inert and
cannot alter guest behavior.

**Address-bearing values.** For every host-contributed address:

- Every `address + size` computation MUST NOT overflow the architecture's
  address width.
- All declared `/memory@*` regions MUST be pairwise non-overlapping with every
  base-DTB node bearing a `reg` property.
- CPU-node `reg` is an identifier (`/cpus` declares `#size-cells = 0`) — the
  MPIDR on aarch64, the APIC ID on x86-64 — not an address. It occupies no
  physical address space and is not subject to the overlap rules above, only to
  uniqueness among CPU nodes. The one address-bearing CPU property,
  `cpu-release-addr`, is specific to the aarch64 `enable-method = "spin-table"`,
  which this extension does not use, so it does not appear. A platform adding a
  spin-table mechanism MUST subject `cpu-release-addr` to the same bounds and
  non-overlap checks as `/memory@*`.

### Non-validation

The merger is NOT required to validate:

- The values of `numa-node-id` properties beyond structural type conformance.
- The values within the `distance-matrix` property under `/distance-map`.
- The values of CPU-node properties (`compatible`, `enable-method`, `reg`,
  `status`) beyond what boot requires. The guest derives real CPU identity and
  features from `MIDR_EL1`/`CPUID` and the attestation report, and must not rely
  on the DTB for them.

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
