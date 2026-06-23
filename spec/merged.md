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
- the base DTB declares a `/cpus` node (or any CPU instance). CPUs are
  host-provided resources: the base declares nothing CPU-related, and the host
  overlay authors the entire `/cpus` subtree — the `/cpus` node itself (with
  `#address-cells`/`#size-cells`) and every `cpu@N` (with its count and per-CPU
  `reg`, `status`, `enable-method`, and `compatible`) — see
  [§2 Allowlist](#allowlist).

The VMM reads the base DTB from the PE file to inform overlay generation. The
image author chooses whether to also place it in guest memory via a `load`
action; if loaded, the bytes contribute to the launch measurement on
confidential targets like any other load. If not loaded, the in-guest consumer
obtains its measured copy of the base DTB by other means (typically by embedding
it in the measured consumer binary).

The base DTB carries the platform definition the image **declares** for the
guest — the device MMIO map, interrupt controller, transport choice, and device
topology. These are image-chosen, not host-discovered: the guest reads its
platform from this measured blob, and the VMM must build a VM that matches it
(see [Platform conformance](#platform-conformance) below). It declares nothing
CPU-related. The `/cpus` subtree and the resource-allocation
surfaces — the `/cpus` container (with `#address-cells`/`#size-cells`) and every
`cpu@N` node (with its `reg`, `status`, `enable-method`, and `compatible`),
`/memory@*`, `/distance-map`, and per-node `numa-node-id` — come from the overlay
in [§2](#2-new-fill-kind-mergeddtbo).

A device `reg` region declared in the base DTB MUST NOT fall within the
2 MiB-aligned region occupied by any `load` or `fill` section (see [Page
Granularity](granularity.md)).

### Platform conformance

The guest derives its entire platform from the measured base DTB (plus the
allowlisted overlay of [§2](#2-new-fill-kind-mergeddtbo)), never from the host. A
VMM that launches the image MUST therefore instantiate that platform: every
device MMIO region, the interrupt controller, and the transport the base DTB
declares MUST be present at the declared addresses. The VMM cannot relocate or
substitute them — a host that emulates a divergent platform does not change the
guest's view; the guest simply finds its expected devices absent and fails to
boot (a denial of service, not a platform substitution). A VMM that cannot
deliver the declared platform MUST refuse to launch.

### PCI passthrough

PCI is enumerable, so it fits this model with no overlay device nodes. The base
DTB declares the PCI host bridge — its ECAM region, MMIO/IO windows, and
MSI/interrupt routing — as ordinary image-owned platform definition. Individual
PCI devices, emulated or host-assigned (VFIO), are not declared in the DTB; the
guest discovers them by enumerating config space, and their BARs are assigned
within the host bridge's image-declared windows. Passthrough therefore reuses the
image-declared host bridge — the host attaches a device, the guest enumerates it
— and needs no second root complex. Because the host bridge is part of the
measured base DTB, the platform stays portable; establishing trust in an assigned
device (e.g. via TDISP on confidential targets) is a separate concern, out of
scope for PMI.

For NUMA-affine assignment, the image declares the additional root complex(es) it
supports in the base DTB, and the host overlay adds `numa-node-id` to the
relevant host-bridge node ([§2 Allowlist](#allowlist)); every device enumerated
under that bridge inherits its NUMA node. Affinity granularity is therefore the
root complex — to span N NUMA nodes the image pre-declares N host bridges, as it
provisions CPUs up to a bound.

What this model cannot absorb is host-variable, **non-enumerable** platform
devices — MMIO devices the guest cannot discover and the image did not declare.
Admitting those would return platform choice to the host and break the
measurement's portability; they remain out of scope.

## 2. New `fill` kind: `merged:dtbo`

The VMM fills the section with a host-supplied flattened devicetree overlay
(DTBO), in the format defined by the [Devicetree Specification][devicetree] v0.4
or later. The host selects the overlay content via VMM-defined input, out of
scope for PMI. The overlay is **unmeasured** — it does not contribute to the
target's launch measurement.

Because the overlay is unmeasured, the guest MUST validate and merge it only from
memory the host cannot mutate after the check — i.e. private memory — never from
host-mutable shared memory in place. Two placements satisfy this:

- **Unmeasured-private** (preferred where the target supports it): the VMM places
  the overlay in private, content-unmeasured guest memory. It is immutable after
  launch, so the guest validates it in place; no copy is needed.
- **Shared** (fallback): the VMM places the overlay in shared memory; the guest
  MUST copy it, in a single pass, into private memory and then validate and merge
  the private copy. The copy is what makes the validated bytes host-immutable.

Per target: SEV places the overlay with `SNP_LAUNCH_UPDATE` `PAGE_TYPE_UNMEASURED`
(unmeasured-private). TDX places it via `KVM_TDX_INIT_MEM_REGION` with the measure
flag clear (private, content unmeasured — the GPA still enters MRTD, which is
deterministic). CCA has no host-content unmeasured-private primitive
(`RMI_DATA_CREATE` measures; `RMI_DATA_CREATE_UNKNOWN` carries no content), so the
overlay is delivered in shared (NS) memory and the realm copies it into private
memory before validating. `vm` has no encryption, so the overlay is ordinary
guest memory and the threat does not apply.

The overlay is adversarial input from the host (the `consumer` is the in-guest
merger). The guest MUST parse the overlay, validate it, and merge it onto the
base DTB named by `merged:dtb` before relying on the platform description. The
merger's implementation is out of scope for this spec.

Requirements on the overlay fall into two classes, by failure mode.

**Validated by the merger (security-relevant).** The merger MUST enforce these
and reject the launch on any violation; because the input is adversarial, the
merger MUST fail closed — rejecting, never crashing:

- the overlay parses as a well-formed FDT v17 overlay (a sequence of top-level
  `fragment@N` nodes, each carrying `target` or `target-path` and an
  `__overlay__` subnode); malformed input is rejected;
- the overlay is within a bounded size the merger accepts (reject oversized — a
  resource-exhaustion defense; recommended bound ≤ 64 KiB);
- the [Allowlist](#allowlist): every contributed node and property falls into an
  allowed category;
- the [Address-bearing values](#address-bearing-values) rules.

Phandles are resolved as part of merging; an unresolvable phandle makes the merge
fail closed.

**Not validated by the merger (denial-of-service only).** A cooperative VMM
provides these so the guest can boot; the merger does not check them, because
their only failure mode is the guest's own non-boot — which a host can cause
regardless (see [Measured vs. host-controlled
inputs](core.md#measured-vs-host-controlled-inputs)):

- the merged DTB's usable RAM — the union of `reg` entries on nodes with
  `device_type = "memory"` whose `status` is absent or `"okay"`, minus any range
  covered by a `/reserved-memory` child carrying the `no-map` property — covers
  every load/fill range in the active target's `actions`, including the sections
  holding the base DTB and the overlay;
- host-contributed `/memory@*` regions do not overlap each other.

### Validation

The merger MUST reject the launch on any of the following security-relevant
checks (the denial-of-service-only requirements above are not checked).

#### Allowlist

Every node and property the overlay contributes MUST fall into one of the
following four categories. Anything outside is non-conformant.

1. The `/cpus` subtree: the overlay authors it in full. It creates the `/cpus`
   node — carrying only `#address-cells`/`#size-cells` — and MAY add `cpu@N`
   nodes for any `N` (the base contains no `/cpus`). Each `cpu@N`'s properties —
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

#### Address-bearing values

For every host-contributed address:

- Every host-contributed address, and every `address + size`, MUST lie within
  the guest's physical address width without overflow — the x86-64
  guest-physical width or the aarch64 IPA width, taken from the architectural or
  target source: `CPUID Fn8000_0008_EAX` (x86-64; reduced by the SEV
  memory-encryption reduction in `Fn8000_001F_EBX` under SEV), the TD `GPAW`
  from `TDCALL[TDG.VP.INFO]` (TDX), `ID_AA64MMFR0_EL1.PARange` (aarch64 `vm`), or
  the realm IPA width from `RSI_REALM_CONFIG` (CCA). The bound is never a
  hardcoded constant.
- All declared `/memory@*` regions MUST be pairwise non-overlapping with every
  base-DTB node bearing a `reg` property.
- CPU-node `reg` is an identifier (the overlay-authored `/cpus` declares
  `#size-cells = 0`) — the
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
    {"type": "load", "gpa": 0x100000,  "section": ".linux"},
    {"type": "load", "gpa": 0x1000000, "section": ".initrd"},
    {"type": "load", "gpa": 0x2000000, "section": ".cmdline"},
    {"type": "load", "gpa": 0x2001000, "section": ".dtb"},
    {"type": "fill", "gpa": 0x2011000, "section": ".dtbo", "kind": "merged:dtbo"}
  ]
}
```
