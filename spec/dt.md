# `dt` Extension

**Prefix:** `dt`.

The `dt` extension describes the guest's platform with a flattened devicetree.
It uses two channels with different trust models:

- a base DTB, a devicetree blob the guest treats as authoritative. The base is
  always measured: whatever the guest receives as the base enters the target's
  launch measurement. Each launch uses exactly one base DTB.
- an optional resource overlay, a devicetree overlay (DTBO) the host supplies to
  allocate the resources (CPUs, memory, NUMA) that the base leaves open. The
  overlay is the only unmeasured channel. It is adversarial input; the guest
  validates it and merges it onto the base, rejecting the launch on any
  violation.

Everything the guest relies on for correctness is either in the measured base or
validated before use. The host's only unvalidated influence is resource
allocation, whose worst case is denial of service.

It defines three extension points:

1. The new target attribute [`dt:dtb`](#1-new-target-attribute-dtdtb).
2. The new `fill` kind [`dt:dtb`](#2-new-fill-kind-dtdtb).
3. The new `fill` kind [`dt:dtbo`](#3-new-fill-kind-dtdtbo).

The base DTB reaches guest memory in one of two ways. No `load` is treated
specially:

- **Loaded.** The base is measured image bytes placed by an ordinary
  [`default` load](core.md#load). This covers both a dedicated section and bytes
  embedded in the measured consumer; PMI does not distinguish the two. The
  consumer locates the base by convention, and the host cannot substitute it.
- **Filled.** A [`dt:dtb` fill](#2-new-fill-kind-dtdtb) has the VMM write the
  measured base into a reserved region. The bytes come from a bundled copy (a
  default the VMM MAY substitute) or, in detached mode, entirely from the VMM.

The optional [`dt:dtb` attribute](#1-new-target-attribute-dtdtb) is orthogonal to
delivery. It exposes a bundled base *section* to the VMM, either to author an
overlay against or to serve as a fill's default source. It places nothing in
guest memory.

See
[Motivation §2](motivation.md#splitting-platform-definition-from-resource-allocation)
for the trust model.

## 1. New target attribute: `dt:dtb`

The `dt:dtb` target attribute names the PE section that holds the bundled base
DTB:

```cddl
dt-dtb = tstr                        ; PE section name
```

The attribute exposes a bundled base section to the VMM. The VMM reads that
section to author a [`dt:dtbo`](#3-new-fill-kind-dtdtbo) overlay against it, and
uses it as the default source of a [`dt:dtb` fill](#2-new-fill-kind-dtdtb). The
attribute is not a delivery path: it places nothing in guest memory, and it
carries no overlay or trust meaning by itself. When it is absent, no bundled base
is exposed to the VMM, and any base the VMM must produce comes from a
[`dt:dtb` fill](#2-new-fill-kind-dtdtb). This is called detached mode.

The guest can only use a base that is present in its memory. The base MUST be
placed there by a [`load`](core.md#load) or by a
[`dt:dtb` fill](#2-new-fill-kind-dtdtb). (A `load` covers both a dedicated
section and bytes embedded in the measured consumer; PMI does not distinguish the
two.) An image that places no base at all fails to boot. That is a denial of
service and not a security defect (see [Enforcement](#enforcement)).

A VMM MUST refuse to launch on any of:

- the section named by `dt:dtb` is not a PE section present in the image;
- the bytes at the named section do not parse as a well-formed flattened
  devicetree blob in the format defined by the [Devicetree
  Specification][devicetree] v0.4 or later.

The base DTB must also satisfy the [resource-ownership
rules](#resource-ownership), which bind it however it is delivered.

The base DTB carries the platform definition the image declares for the guest:
the device MMIO map, interrupt controller, transport choice, and device topology.
The image chooses these; the host does not discover them. The guest reads its
platform from the measured base, and the VMM must build a VM that matches it (see
[Platform conformance](#platform-conformance)). Which resources (CPUs, memory,
NUMA) the base fixes, and which it leaves to the overlay, is governed by
[Resource ownership](#resource-ownership).

A device `reg` region declared in the base DTB MUST NOT fall within the
2 MiB-aligned region occupied by any `load` or `fill` section (see [Page
Granularity](granularity.md)).

## 2. New `fill` kind: `dt:dtb`

The `dt:dtb` fill kind delivers the measured base DTB into a reserved region.
As with every [`fill`](core.md#fill), the action's `section` MUST be a Zero
section: it reserves the guest-physical range and its size, and holds no image
bytes. The VMM populates that range with a base DTB and measures the content.
(The [`dt:dtbo`](#3-new-fill-kind-dtdtbo) overlay, by contrast, is never
measured.)

```cbor-diag
{"type": "fill", "gpa": 0x2001000, "section": ".dtb", "kind": "dt:dtb"}
```

The bytes the VMM writes come from one of two sources:

- the bundled base named by the [`dt:dtb`](#1-new-target-attribute-dtdtb)
  attribute, used as a non-authoritative default that the VMM MAY replace with a
  different DTB; or
- in detached mode (no attribute), a base DTB the VMM supplies entirely.

The bundled base never takes precedence over a substitute. An image that needs an
exact, non-substitutable base bundles it and places it with a
[`default` load](core.md#load) rather than this fill. A `default` load places the
section's bytes verbatim, so the host cannot substitute them. This fill's section
is a Zero section (no bytes), whereas a bundled base is a Data section (bytes), so
the two are always different sections.

A VMM MUST refuse to launch if the base DTB it delivers exceeds the reserved
section size, or does not parse as a well-formed FDT ([Devicetree
Specification][devicetree] v0.4 or later).

The base is measured, per target:

- **`vm`**: ordinary guest memory (no measurement on this target).
- **`sev`**: `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_NORMAL` (measured into the
  launch digest).
- **`tdx`**: `KVM_TDX_INIT_MEM_REGION` with `KVM_TDX_MEASURE_MEMORY_REGION` set
  (`TDH.MEM.PAGE.ADD` then `TDH.MR.EXTEND` into MRTD).
- **`cca`**: `RMI_DATA_CREATE` (measured).

Because the host MAY substitute the base, the measurement records what the guest
actually received. Whether that value is *predictable* depends on who authored
the bytes; see [Authorship and attestation
predictability](#authorship-and-attestation-predictability).

## 3. New `fill` kind: `dt:dtbo`

The `dt:dtbo` fill kind delivers a host-supplied devicetree overlay (DTBO) into a
Zero section. The overlay uses the format defined by the [Devicetree
Specification][devicetree] v0.4 or later. The host selects the overlay content
through VMM-defined input, which is out of scope for PMI. The overlay is
unmeasured: it does not contribute to the target's launch measurement.

```cbor-diag
{"type": "fill", "gpa": 0x2011000, "section": ".dtbo", "kind": "dt:dtbo"}
```

A `dt:dtbo` overlay is meaningless without a base to merge onto. The image
author MUST place a base in guest memory with a [`load`](core.md#load) or a
[`dt:dtb` fill](#2-new-fill-kind-dtdtb) (see [§1](#1-new-target-attribute-dtdtb)).
The VMM does not check this at launch, because a `load`ed base is opaque to it
and it cannot in general tell whether a base is present. Enforcement falls to the
in-guest merger, which rejects the launch when it has nothing to merge onto (a
denial of service). The VMM needs to *read* the base, through the attribute or a
fill, only when the overlay decorates existing base nodes (for example
`numa-node-id`). A fresh `/cpus` + `/memory@*` + `/distance-map` overlay needs no
knowledge of the base.

Because the overlay is unmeasured, the guest MUST validate and merge it only from
memory the host cannot mutate after the check (that is, private memory), never
host-mutable shared memory in place. Two placements satisfy this:

- **Unmeasured-private** (preferred where the target supports it): the VMM places
  the overlay in private, content-unmeasured guest memory. It is immutable after
  launch, so the guest validates it in place and needs no copy.
- **Shared** (fallback): the VMM places the overlay in shared memory. The guest
  MUST copy it, in a single pass, into private memory, then validate and merge
  the private copy. The copy is what makes the validated bytes host-immutable.

Per target:

- **`vm`**: ordinary guest memory; there is no encryption, so the threat does not
  apply.
- **`sev`**: `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_UNMEASURED` (unmeasured-private).
- **`tdx`**: `KVM_TDX_INIT_MEM_REGION` with the measure flag clear (private,
  content unmeasured; the GPA still enters MRTD, which is deterministic).
- **`cca`**: no host-content unmeasured-private primitive exists, since
  `RMI_DATA_CREATE` measures and `RMI_DATA_CREATE_UNKNOWN` carries no content.
  The overlay is therefore delivered in shared (NS) memory, and the realm copies
  it into private memory before validating.

The overlay is adversarial input from the host. Before the guest relies on the
platform description, the in-guest merger MUST parse the overlay, validate it, and
merge it onto the base. The merger's implementation is out of scope for this
spec. Requirements on the overlay appear in [Overlay
validation](#overlay-validation).

## Resource ownership

CPUs, memory, and NUMA are owned independently. Each of CPUs and memory is
authored in full by exactly one party: either the tenant, in the measured base,
or the host, in the overlay. Neither resource is ever split between them.
Whenever an overlay exists, NUMA is the host's to decide.

| Resource | No overlay | Overlay present, base declares it | Overlay present, base omits it |
|---|---|---|---|
| **CPUs** (`/cpus`) | base MUST declare it (fixed, measured) | fixed, measured; overlay MUST NOT author `/cpus`/`cpu@N` | overlay authors `/cpus` in full |
| **Memory** (`/memory@*`) | base MUST declare it (fixed, measured) | fixed, measured; overlay MUST NOT author `/memory@*` | overlay authors memory |
| **NUMA** (`/distance-map`, `numa-node-id`) | base MAY declare it (measured) | base MUST NOT declare it; overlay MAY author it | base MUST NOT declare it; overlay MAY author it |

Consequences:

- A resource the base declares is fixed: exact and immutable by the host. A
  base that fixes CPUs pins an exact count. Host CPU scaling ("up to a bound") is
  available only when the base omits `/cpus` and the overlay authors it.
- When an overlay is present, NUMA is always the host's decision, even where the
  base fixes both CPUs and memory. The host adds a `/distance-map` and attaches
  `numa-node-id` to the base's `cpu@N` and `memory@` nodes. A NUMA-only overlay,
  where the base fixes CPUs and memory and the overlay contributes only
  `/distance-map` and `numa-node-id`, is valid.
- NUMA affinity is a *placement* decision that only the host can make. Getting it
  wrong degrades performance rather than correctness or security, so it belongs
  on the unmeasured channel. When there is no overlay the system is fully static,
  and any NUMA topology legitimately lives in the measured base.
- Base-fixed CPUs and memory are attested; the NUMA affinity supplied by the
  overlay is not. A verifier sees the resource layout but not its NUMA placement.

The base DTB declares nothing CPU-related unless it fixes CPUs. When CPUs are
host-authored, the overlay creates the `/cpus` node itself, with
`#address-cells`/`#size-cells`, and every `cpu@N` (see the
[Allowlist](#allowlist)).

These rules bind the base DTB however it is delivered, whether bundled and loaded
or written by a [`dt:dtb` fill](#2-new-fill-kind-dtdtb). A VMM MUST refuse to
launch when the base it delivers omits `/cpus` or memory while no overlay is
present, or declares any NUMA (`/distance-map` or a `numa-node-id` property) while
an overlay is present. The complementary constraint, that the overlay MUST NOT
author a resource the base fixes, is enforced by the in-guest merger (see
[Overlay validation](#overlay-validation) and [Enforcement](#enforcement)).

## Authorship and attestation predictability

The base DTB is always measured, whichever way it is delivered and whoever wrote
it. Measurement records what the guest received; it does not constrain who chose
those bytes. What determines whether the measurement is *predictable*, and
therefore whether attestation can be appraised, is authorship. Authorship is a
separate axis from delivery.

Substitution is a separate matter from authorship. A
[`dt:dtb` fill](#2-new-fill-kind-dtdtb) lets the VMM place a base that differs
from the bundled default, and that substitution is a supported, first-class
capability. The question that matters is only *who authored the bytes the VMM
delivered*.

- **Tenant-authored (the intended case).** The tenant controls the base DTB
  bytes. Because the base is measured, the tenant can precompute the expected
  launch measurement and appraise attestation. This holds whether the base is
  bundled (loaded, or used as a `dt:dtb` fill default) or delivered detached. The
  intended detached flow is exactly this: the VMM substitutes the bundled base
  with an out-of-band, tenant-authored DTB. Substitution is expected here; the
  tenant has simply authored the DTB somewhere other than the PMI bundle. That is
  the purpose of detached mode. It decouples DTB *distribution* from PMI
  distribution (one image, many separately shipped tenant DTBs) while keeping the
  tenant as the author.

- **Host- or VMM-authored (permitted, strongly discouraged).** The `dt:dtb` fill
  technically lets the VMM author the base itself. The result is still measured,
  but the tenant can no longer predict the bytes, so the measurement becomes
  fragile: it varies with host choice and cannot be appraised in advance. The
  mechanism allows this, but does not endorse it. Images and deployments SHOULD
  keep the base tenant-authored.

## Enforcement

Three distinct mechanisms uphold this extension's guarantees. An implementer
MUST NOT mistake one for another.

- **VMM checks.** The "A VMM MUST refuse to launch …" conditions above let a
  cooperative host fail fast on a malformed image. On confidential targets the
  VMM is untrusted, so these checks are advisory. A malicious VMM can ignore
  them, but the worst it achieves is a guest that cannot boot, which is a denial
  of service.
- **Measurement.** The base DTB is measured, so a wrong or substituted base
  changes the launch measurement and is caught at attestation. This, rather than
  the VMM check, is what makes the base trustworthy on confidential targets. A
  guest relies on the base only as far as a remote verifier appraises the
  measurement (see [Authorship and attestation
  predictability](#authorship-and-attestation-predictability) and [Measured vs.
  host-controlled inputs](core.md#measured-vs-host-controlled-inputs)).
- **The in-guest merger.** The overlay is unmeasured and adversarial, so nothing
  above protects it. The merger is its only security boundary. It MUST validate
  the overlay against the [Allowlist](#allowlist) and merge it fail-closed (see
  [Overlay validation](#overlay-validation)).

## Overlay validation

Requirements on the overlay fall into two classes, by failure mode.

**Validated by the merger (security-relevant).** The merger MUST enforce these
and reject the launch on any violation. Because the input is adversarial, the
merger MUST fail closed: it rejects rather than crashes.

- the overlay parses as a well-formed FDT v17 overlay: a sequence of top-level
  `fragment@N` nodes, each carrying `target` or `target-path` and an
  `__overlay__` subnode. Malformed input is rejected;
- the overlay is within a bounded size the merger accepts. Oversized input is
  rejected, as a resource-exhaustion defense; the recommended bound is ≤ 64 KiB;
- the [Allowlist](#allowlist): every contributed node and property falls into an
  allowed category, and no category authors a resource the base already fixed;
- the [Address-bearing values](#address-bearing-values) rules.

Phandles are resolved as part of merging. An unresolvable phandle makes the merge
fail closed.

**Not validated by the merger (denial-of-service only).** A cooperative VMM
provides these so the guest can boot. The merger does not check them, because
their only failure mode is the guest's own non-boot, which a host can cause
regardless (see [Measured vs. host-controlled
inputs](core.md#measured-vs-host-controlled-inputs)):

- the merged DTB's usable RAM covers every load/fill range in the active target's
  `actions`, including the sections holding the base DTB and the overlay. Usable
  RAM is the union of `reg` entries on nodes with `device_type = "memory"` whose
  `status` is absent or `"okay"`, minus any range covered by a `/reserved-memory`
  child carrying the `no-map` property;
- host-contributed `/memory@*` regions do not overlap each other.

### Allowlist

Every node and property the overlay contributes MUST fall into one of the
following four categories. The two resource-authoring categories are permitted
only when the base leaves that resource open. Anything outside these categories
is non-conformant, and the merger MUST reject it.

1. The `/cpus` subtree, allowed only if the base declares no `/cpus`. When
   permitted, the overlay authors it in full: it creates the `/cpus` node,
   carrying only `#address-cells`/`#size-cells`, and MAY add `cpu@N` nodes for
   any `N`. Each `cpu@N`'s properties are host-authored: `device_type`
   (= `"cpu"`), `reg`, and any of `status`, `enable-method`, `compatible`. The
   overlay MUST NOT set `phandle` or `linux,phandle`. The total CPU count MUST be
   bounded (recommended ≤ a VMM-defined maximum) to prevent resource exhaustion.
   If the base declares `/cpus`, the overlay MUST NOT contribute `/cpus` or any
   `cpu@N`; it MAY only attach `numa-node-id` to an existing `cpu@N`, per
   category 4.

   The CPUs are homogeneous in identity and bringup, so the overlay SHOULD give
   every `cpu@N` the same `compatible` and `enable-method`. `status` is per-CPU:
   the boot CPU MUST be `okay`, while others MAY be `disabled` (for example,
   offline-capable or hot-onlineable). Each `reg` MUST be unique. Malformed or
   inconsistent CPU nodes (a missing or duplicate `reg`, a non-`okay` boot CPU,
   or an `enable-method` the platform does not implement) manifest as boot
   failures (DoS) and are not consumer-validated. Per-CPU divergence in identity
   (heterogeneous topology) is out of scope for this extension.

2. Nodes and properties under `/memory@*`, allowed only if the base declares no
   memory (no node with `device_type = "memory"`). If the base declares memory,
   the overlay MUST NOT contribute `/memory@*`; it MAY only attach `numa-node-id`
   to an existing `memory@` node, per category 4.

3. Nodes and properties under `/distance-map`. These are always permitted when an
   overlay is present (NUMA); the base MUST NOT declare `/distance-map`.

4. The `numa-node-id` property added to any node the base DTB already declared.
   This is always permitted when an overlay is present (NUMA); the base MUST NOT
   declare `numa-node-id` on any node. It is the only property the host MAY add
   outside the first three paths, and it MUST NOT appear alongside any other
   host-contributed property on the same node.

**The CPU `compatible` is non-authoritative.** It is host-supplied, unmeasured,
and on confidential targets adversarial. Guests and remote verifiers MUST derive
actual CPU identity and features from the architectural identification registers
(`MIDR_EL1` on aarch64, `CPUID` on x86-64) and, on attested targets, from the
target's attestation report, never from this property. CPU errata are keyed on
the identification registers rather than on `compatible`, so
the value is inert and cannot alter guest behavior.

### Address-bearing values

For every host-contributed address:

- Every host-contributed address, and every `address + size`, MUST lie within
  the guest's physical address width without overflow. That width is the x86-64
  guest-physical width or the aarch64 IPA width, taken from the architectural or
  target source: `CPUID Fn8000_0008_EAX` (x86-64, reduced by the SEV
  memory-encryption reduction in `Fn8000_001F_EBX` under SEV), the TD `GPAW`
  from `TDCALL[TDG.VP.INFO]` (TDX), `ID_AA64MMFR0_EL1.PARange` (aarch64 `vm`), or
  the realm IPA width from `RSI_REALM_CONFIG` (CCA). The bound is never a
  hardcoded constant.
- All declared `/memory@*` regions MUST be pairwise non-overlapping with every
  base-DTB node bearing a `reg` property.
- CPU-node `reg` is an identifier, not an address (the overlay-authored `/cpus`
  declares `#size-cells = 0`): the MPIDR on aarch64, the APIC ID on x86-64. It
  occupies no physical address space and is not subject to the overlap rules
  above, only to uniqueness among CPU nodes. The one address-bearing CPU
  property, `cpu-release-addr`, is specific to the aarch64
  `enable-method = "spin-table"`, which this extension does not use, so it does
  not appear. A platform that adds a spin-table mechanism MUST subject
  `cpu-release-addr` to the same bounds and non-overlap checks as `/memory@*`.

### Non-validation

The merger is NOT required to validate:

- The values of `numa-node-id` properties beyond structural type conformance.
- The values within the `distance-matrix` property under `/distance-map`.
- The values of CPU-node properties (`compatible`, `enable-method`, `reg`,
  `status`) beyond what boot requires. The guest derives real CPU identity and
  features from `MIDR_EL1`/`CPUID` and the attestation report, and must not rely
  on the DTB for them.

## Platform conformance

The guest derives its entire platform from the measured base DTB, plus the
allowlisted overlay, and never from the host. A VMM that launches the image MUST
therefore instantiate that platform: every device MMIO region, the interrupt
controller, and the transport the base DTB declares MUST be present at the
declared addresses. The VMM cannot relocate or substitute them. A host that
emulates a divergent platform does not change the guest's view; the guest simply
finds its expected devices absent and fails to boot, which is a denial of service
rather than a platform substitution. A VMM that cannot deliver the declared
platform MUST refuse to launch. This conformance extends to resources: the VMM
MUST construct a VM whose CPUs, memory, and NUMA topology exactly match the
merged result (base plus overlay, where present).

## PCI passthrough

PCI is enumerable, so it fits this model with no overlay device nodes. The base
DTB declares the PCI host bridge (its ECAM region, MMIO/IO windows, and
MSI/interrupt routing) as ordinary image-owned platform definition. Individual
PCI devices, whether emulated or host-assigned (VFIO), are not declared in the
DTB. The guest discovers them by enumerating config space, and their BARs are
assigned within the host bridge's image-declared windows. Passthrough therefore
reuses the image-declared host bridge: the host attaches a device and the guest
enumerates it, with no second root complex needed. Because the host bridge is
part of the measured base DTB, the platform stays portable. Establishing trust in
an assigned device (for example, via TDISP on confidential targets) is a separate
concern and out of scope for PMI.

For NUMA-affine assignment, the image declares the additional root complexes it
supports in the base DTB, and the host overlay adds `numa-node-id` to the
relevant host-bridge node ([Allowlist](#allowlist) category 4). Every device
enumerated under that bridge inherits its NUMA node. Affinity granularity is
therefore the root complex: to span N NUMA nodes the image pre-declares N host
bridges, just as it provisions CPUs up to a bound.

What this model cannot absorb is host-variable, non-enumerable platform devices:
MMIO devices the guest cannot discover and the image did not declare. Admitting
those would return platform choice to the host and break the measurement's
portability, so they remain out of scope.

## Examples

A `.pmi.vm` that loads a kernel, initrd, and command line, bundles a base DTB
that fixes the platform but omits CPUs and memory, and lets the host allocate
them via an overlay. The base is bundled and placed with an ordinary `default`
load, so its bytes are authoritative:

```cbor-diag
{
  "version": 1,
  "vm:vcpu": {"rip": 0x100000, "rsp": 0x80000, "rflags": 0x2},
  "cpu:profile": "x86-64-v2",
  "dt:dtb": ".dtb",
  "actions": [
    {"type": "load", "gpa": 0x100000,  "section": ".linux"},
    {"type": "load", "gpa": 0x1000000, "section": ".initrd"},
    {"type": "load", "gpa": 0x2000000, "section": ".cmdline"},
    {"type": "load", "gpa": 0x2001000, "section": ".dtb"},
    {"type": "fill", "gpa": 0x2011000, "section": ".dtbo", "kind": "dt:dtbo"}
  ]
}
```

The same image in detached mode has no `dt:dtb` attribute, and the base is
delivered by a `dt:dtb` fill into the reserved `.dtb` Zero section. The VMM
conveys an out-of-band, tenant-authored base into it (measured), and the host
allocates CPUs, memory, and NUMA via the overlay:

```cbor-diag
{
  "version": 1,
  "vm:vcpu": {"rip": 0x100000, "rsp": 0x80000, "rflags": 0x2},
  "cpu:profile": "x86-64-v2",
  "actions": [
    {"type": "load", "gpa": 0x100000,  "section": ".linux"},
    {"type": "load", "gpa": 0x1000000, "section": ".initrd"},
    {"type": "load", "gpa": 0x2000000, "section": ".cmdline"},
    {"type": "fill", "gpa": 0x2001000, "section": ".dtb",  "kind": "dt:dtb"},
    {"type": "fill", "gpa": 0x2011000, "section": ".dtbo", "kind": "dt:dtbo"}
  ]
}
```

Per-resource ownership does not change the target shape; it is a property of the
base and overlay *contents*. Using the first example's `actions` unchanged, a
NUMA-only overlay has the base fix both CPUs and memory, so the overlay may touch
neither, while the host contributes only affinity:

```dts
// base .dtb (measured): fixes CPUs and memory, declares no NUMA
/ {
    cpus { #address-cells = <1>; #size-cells = <0>;
        cpu@0 { device_type = "cpu"; reg = <0>; /* … */ };
        cpu@1 { device_type = "cpu"; reg = <1>; /* … */ };
    };
    memory@40000000 { device_type = "memory"; reg = <0x40000000 0x40000000>; };
};

// host .dtbo (unmeasured): NUMA only — /distance-map plus numa-node-id on
// nodes the base already declared (Allowlist categories 3 and 4)
/dts-v1/ /plugin/;
&{/cpus/cpu@0}   { numa-node-id = <0>; };
&{/cpus/cpu@1}   { numa-node-id = <1>; };
&{/memory@40000000} { numa-node-id = <0>; };
/ { distance-map { compatible = "numa-distance-map-v1";
        distance-matrix = <0 0 10>, <0 1 20>, <1 1 10>; }; };
```

A mixed image applies the same idea one resource at a time: the base declares
`/cpus` (fixing an exact CPU set) but omits `/memory@*`, so the overlay authors
memory and NUMA while leaving CPUs alone.

[devicetree]: https://www.devicetree.org/specifications/
