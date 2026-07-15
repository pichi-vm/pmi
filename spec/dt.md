# `dt` Extension

**Prefix:** `dt`.

The fundamental problem of launching a guest boils down to the negotiation of
the platform configuration. In the traditional VM model, the host would build
whatever platform it wanted and it only had to communicate this to the guest.
This model used ACPI or devicetree to accomplish this.

PMI, however, aims to give the tenant, rather than the host, control of this
process. PMI does this with a simple protocol:

1. the guest tells the host the platform it requires
2. the host complies or fails to boot
3. the host allocates resources (CPUs, memory, NUMA)
4. the guest verifies the allocated resources

PMI implements this protocol using devicetree. The guest will supply a base
DTB to the host and the host, if permitted by the tenant, will generate an
overlay containing allocated resources. On Confidential Computing deployments,
the base DTB is measured, and is thus part of the identity of the guest. In
contrast, to prevent allocated resources from changing guest identity, the overlay
is validated, but never measured.

This extension, therefore, defines the mechanisms used to enact this
negotiation. It gives the tenant two distinct facilities to control:

1. **How does the VMM provide the base DTB to the guest?** This is called the
   **channel** facility. There are three modes of operation: **bundled**,
   **detached**, and **optional**. In **bundled** mode, the base DTB is
   contained within the PMI. In **detached** mode, the base DTB is provided out
   of band. In **optional** mode, the VMM may use an out-of-band base DTB and
   fall back to a bundled DTB if it is not available.

2. **Does the guest permit host allocation of resources?** This is called the
   **allocation** facility. The guest has three resource types it can
   delegate to the VMM: CPUs, memory, and NUMA. Alternatively, it can require
   the host to provide an exact layout.

This extension defines three extension points:

1. The new target attribute [`dt:dtb`](#1-new-target-attribute-dtdtb).
2. The new `fill` kind [`dt:dtb`](#2-new-fill-kind-dtdtb).
3. The new `fill` kind [`dt:dtbo`](#3-new-fill-kind-dtdtbo).

What the producer must build is defined under [Producer](#producer); how the VMM
realizes the channel modes and validates the result under [VMM](#vmm); and
what the guest must do with the overlay under [Guest](#guest).

See
[Motivation §2](motivation.md#splitting-platform-definition-from-resource-allocation)
for the trust model.

## 1. New target attribute: `dt:dtb`

The `dt:dtb` target attribute names the PE section that holds the bundled base
DTB:

```cddl
dt-dtb = tstr                        ; PE section name
```

The attribute exposes a bundled base DTB to the VMM. In **bundled** mode it is
the launch base DTB; in **optional** mode it is the fallback used when no
out-of-band base DTB is provided. The attribute only makes a base DTB available
to the VMM and places nothing in guest memory; how the VMM selects the launch
base DTB is defined under [VMM](#vmm).

### Base resources

The base DTB declares the guest's platform: the device MMIO map, interrupt
controller, transport choice, and device topology. It also partitions the
resources the tenant fixes from those the host may allocate:

- **CPUs** (`/cpus`): declaring `/cpus` fixes the CPU set, exact and measured;
  omitting it delegates CPU allocation to the overlay.
- **Memory** (`/memory@*`): declaring memory fixes it, measured; omitting it
  delegates sizing to the overlay.
- **NUMA** (`/distance-map`, `numa-node-id`): whenever the image ships an
  overlay, NUMA is the host's decision, so the base MUST NOT declare it.

What the overlay may contribute for a delegated resource is defined under
[Overlay contents](#overlay-contents).

## 2. New `fill` kind: `dt:dtb`

The `dt:dtb` fill kind delivers a base DTB into a reserved Zero section. The VMM
populates that section and measures its content (see [VMM](#vmm)).

```cbor-diag
{"type": "fill", "gpa": 0x2001000, "section": ".dtb", "kind": "dt:dtb"}
```

## 3. New `fill` kind: `dt:dtbo`

The `dt:dtbo` fill kind delivers a host-supplied devicetree overlay (DTBO), in
the format defined by the [Devicetree Specification][devicetree] v0.4 or later,
into a reserved Zero section.

```cbor-diag
{"type": "fill", "gpa": 0x2011000, "section": ".dtbo", "kind": "dt:dtbo"}
```

The overlay is unmeasured and allocates the resources the base leaves open (CPUs,
memory, and NUMA), so that host resource choices do not change the guest's
identity. The VMM places it (see [VMM](#vmm)) and the guest validates it (see
[Guest](#guest)).

### Overlay contents

The overlay is the resource-allocation channel: it carries CPU, memory, and NUMA
allocation and nothing else. This definition is normative for all three actors:
the [producer](#producer) authors the base so that every resource it delegates is
left open to the overlay; the [VMM](#vmm) populates the overlay with only the
content defined here; and the [guest](#guest) rejects any overlay that goes beyond
it.

Every node and property the overlay contributes MUST fall into one of the four
categories below. A category that authors a resource (CPUs or memory) is
permitted only when the base leaves that resource open (see [Base
resources](#base-resources)): by declaring a resource in the base, the tenant
denies the host the opportunity to specify it in the overlay.

1. The `/cpus` subtree, permitted only if the base declares no `/cpus`. When
   permitted, the overlay authors it in full: it creates the `/cpus` node,
   carrying only `#address-cells`/`#size-cells`, and MAY add `cpu@N` nodes for
   any `N`. Each `cpu@N`'s properties are host-authored: `device_type`
   (= `"cpu"`), `reg`, and any of `status`, `enable-method`, or `compatible`. The
   overlay MUST NOT set `phandle` or `linux,phandle`. The total CPU count MUST be
   bounded (recommended ≤ an implementation-defined maximum) to prevent resource
   exhaustion. If the base declares `/cpus`, the overlay MUST NOT contribute
   `/cpus` or any `cpu@N`; it MAY only attach `numa-node-id` to an existing
   `cpu@N`, per category 4.

   The CPUs are homogeneous in identity and bringup, so the overlay SHOULD give
   every `cpu@N` the same `compatible` and `enable-method`. `status` is per-CPU:
   the boot CPU MUST be `okay`, while others MAY be `disabled` (for example,
   offline-capable or hot-onlineable). Each `reg` MUST be unique.

2. Nodes and properties under `/memory@*`, permitted only if the base declares no
   memory (no node with `device_type = "memory"`). If the base declares memory,
   the overlay MUST NOT contribute `/memory@*`; it MAY only attach `numa-node-id`
   to an existing `memory@` node, per category 4.

3. Nodes and properties under `/distance-map` (NUMA), always permitted when an
   overlay is present.

4. The `numa-node-id` property added to any node the base DTB already declared
   (NUMA), always permitted when an overlay is present. It is the only property
   the host MAY add outside the first three categories, and it MUST NOT appear
   alongside any other host-contributed property on the same node.

**The CPU `compatible` is non-authoritative.** It is host-supplied, unmeasured,
and on confidential targets adversarial. Guests and remote verifiers MUST derive
actual CPU identity and features from the architectural identification registers
(`MIDR_EL1` on aarch64, `CPUID` on x86-64) and, on attested targets, from the
target's attestation report, never from this property.

## Producer

A PMI producer MUST:

- provide a base DTB in one of the channel modes:
  - **bundled**: place the base in a section, name it with the
    [`dt:dtb`](#1-new-target-attribute-dtdtb) attribute, and deliver it with a
    [`default` load](core.md#load);
  - **detached**: reserve a Zero section for the base and add a
    [`dt:dtb` fill](#2-new-fill-kind-dtdtb) action naming it, with no attribute;
  - **optional**: set the [`dt:dtb`](#1-new-target-attribute-dtdtb) attribute
    (the fallback base) and add the [`dt:dtb` fill](#2-new-fill-kind-dtdtb)
    action;
- author the base DTB per [Base resources](#base-resources): the platform
  definition, and the choice to fix or delegate CPUs, memory, and NUMA;
- if it delegates any resource, reserve a Zero section for the overlay and add a
  [`dt:dtbo` fill](#3-new-fill-kind-dtdtbo) action naming it;
- size each reserved Zero section for the largest DTB it will hold;
- lay out sections so that no device `reg` region in the base falls within the
  2 MiB-aligned region of any [`load`](core.md#load) or [`fill`](core.md#fill)
  section (see [Page Granularity](granularity.md)).

To keep attestation predictable, the base SHOULD be tenant-authored (see
[Authorship and attestation
predictability](#authorship-and-attestation-predictability)).

## VMM

The `dt` extension participates in each target's launch model. This section
defines the VMM's behavior: how it selects the launch base DTB, places and
measures it, and places the overlay.

The VMM selects the launch base DTB from the presence of the
[`dt:dtb`](#1-new-target-attribute-dtdtb) attribute and the
[`dt:dtb` fill](#2-new-fill-kind-dtdtb) action:

| `dt:dtb` attribute | `dt:dtb` fill | Mode     | Launch base DTB                                                                                     |
| ------------------ | ------------- | -------- | --------------------------------------------------------------------------------------------------- |
| present            | absent        | bundled  | the attribute's section, placed by a [`default` load](core.md#load)                                 |
| absent             | present       | detached | a base the VMM supplies out-of-band, written by the fill                                            |
| present            | present       | optional | an out-of-band base if the VMM has one, otherwise the attribute's bundled base, written by the fill |
| absent             | absent        | invalid  | no base is available; the VMM MUST refuse to launch                                                 |

When the VMM supplies an out-of-band base DTB (in detached mode, and in optional
mode when it has one), it MAY even author that base itself, though the launch
measurement is then unpredictable (see [Authorship and attestation
predictability](#authorship-and-attestation-predictability)).

The VMM places the launch base DTB and folds it into the target's launch
measurement, exactly as it measures a [`default` load](core.md#load). A loaded
base reaches guest memory as an ordinary [`default` load](core.md#load); a filled
base as a [`dt:dtb` fill](#2-new-fill-kind-dtdtb).

If a [`dt:dtbo` fill](#3-new-fill-kind-dtdtbo) is present, the VMM places the
overlay, unmeasured, in memory the host cannot mutate after launch: private,
content-unmeasured memory on targets with memory encryption, or ordinary guest
memory otherwise. The overlay it supplies MUST contain only the content defined
under [Overlay contents](#overlay-contents); because the overlay is unmeasured,
the guest, not the VMM, enforces this (see [Guest](#guest)).

Each target's spec defines the firmware primitives that realize the measured
base placement and the unmeasured-private overlay placement.

A VMM MUST refuse to launch on any of:

- neither the `dt:dtb` attribute nor a `dt:dtb` fill action is present;
- the section named by the `dt:dtb` attribute is not a PE section present in the
  image;
- the launch base DTB does not parse as a well-formed flattened devicetree blob
  in the format defined by the [Devicetree Specification][devicetree] v0.4 or
  later;
- a `dt:dtb` fill delivers a base DTB larger than its reserved section;
- a device `reg` region declared in the base DTB falls within the 2 MiB-aligned
  region occupied by any [`load`](core.md#load) or [`fill`](core.md#fill) section
  (see [Page Granularity](granularity.md)).

On confidential targets the VMM is untrusted, so these checks are advisory: a
cooperative host fails fast on a malformed image, but a malicious host can skip
them, causing at worst a guest that cannot boot (a denial of service). The base
DTB's trustworthiness rests on its measurement, not on these checks; the
overlay's rests on [Guest](#guest).

## Guest

The base DTB is measured and authoritative: the guest relies on it as far as a
remote verifier appraises the launch measurement (see [Authorship and attestation
predictability](#authorship-and-attestation-predictability)). The overlay is
unmeasured and adversarial, so the guest is its sole security boundary and MUST
validate it before relying on the platform description.

The guest MUST:

- validate and merge the overlay only from memory the host cannot mutate after
  the check. The VMM places the overlay in private, content-unmeasured memory
  (see [VMM](#vmm)), which is immutable after launch, so the guest validates it
  in place;
- reject malformed or disallowed input by halting (a denial of service) rather
  than proceeding or crashing;
- accept only the content defined under [Overlay contents](#overlay-contents),
  rejecting any overlay that contributes anything else.

An overlay is meaningless without a base to merge onto; if none is present the
merge fails (a denial of service). How the guest parses and merges the overlay is
out of scope.

## Authorship and attestation predictability

The base DTB is always measured, however it is delivered. Measurement records
what the guest received; it does not fix who chose the bytes. A substituted base
changes the launch measurement and is caught at attestation, but the measurement
is only _predictable_, and attestation only appraisable in advance, when the
base is **tenant-authored**. Detached mode exists to keep it so: it decouples DTB
distribution from PMI distribution (one image, many separately shipped tenant
DTBs) while the tenant remains the author. If the VMM instead authors the base,
the measurement varies with host choice and cannot be appraised in advance. This
is why the [Producer](#producer) keeps the base tenant-authored.

## Examples

A `.pmi.vm` that loads a kernel, initrd, and command line, and bundles a base DTB
placed with an ordinary `default` load so its bytes are authoritative (bundled).
The host allocates CPUs, memory, and NUMA via the overlay:

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

The same image in detached mode: no `dt:dtb` attribute, and the base is delivered
by a `dt:dtb` fill into the reserved `.dtb` Zero section. The VMM conveys an
out-of-band, tenant-authored base into it (measured):

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

[devicetree]: https://www.devicetree.org/specifications/
