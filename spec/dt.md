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
  validates it and merges it onto the base before relying on it.

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
  [`default` load](core.md#load), whether a dedicated section or bytes embedded
  in the measured consumer (PMI does not distinguish the two). The host cannot
  substitute it.
- **Filled.** A [`dt:dtb` fill](#2-new-fill-kind-dtdtb) has the VMM write the
  measured base into a reserved region, from a bundled copy (a default the VMM
  MAY substitute) or, in detached mode, entirely from the VMM.

The optional [`dt:dtb` attribute](#1-new-target-attribute-dtdtb) is orthogonal to
delivery. It exposes a bundled base *section* to the VMM, to author an overlay
against or to serve as a fill's default source. It places nothing in guest
memory. When it is absent, the mode is **detached**: any base the VMM produces
comes from a [`dt:dtb` fill](#2-new-fill-kind-dtdtb).

See
[Motivation §2](motivation.md#splitting-platform-definition-from-resource-allocation)
for the trust model.

## 1. New target attribute: `dt:dtb`

The `dt:dtb` target attribute names the PE section that holds the bundled base
DTB:

```cddl
dt-dtb = tstr                        ; PE section name
```

The attribute exposes a bundled base section to the VMM: the VMM reads it to
author a [`dt:dtbo`](#3-new-fill-kind-dtdtbo) overlay against it, and uses it as
the default source of a [`dt:dtb` fill](#2-new-fill-kind-dtdtb). It is not a
delivery path and places nothing in guest memory. When it is absent (detached
mode), no bundled base is exposed and the base comes from a
[`dt:dtb` fill](#2-new-fill-kind-dtdtb).

The guest can only use a base present in its memory, placed there by a
[`load`](core.md#load) or a [`dt:dtb` fill](#2-new-fill-kind-dtdtb). An image
that places no base fails to boot, which is a denial of service, not a security
defect.

A VMM MUST refuse to launch on any of:

- the section named by `dt:dtb` is not a PE section present in the image;
- the bytes at the named section do not parse as a well-formed flattened
  devicetree blob in the format defined by the [Devicetree
  Specification][devicetree] v0.4 or later.

The base DTB carries the platform definition the image declares for the guest:
the device MMIO map, interrupt controller, transport choice, and device topology.
The guest reads its platform from the measured base, and the VMM must build a VM
that matches it. A device `reg` region declared in the base DTB MUST NOT fall
within the 2 MiB-aligned region occupied by any `load` or `fill` section (see
[Page Granularity](granularity.md)).

## 2. New `fill` kind: `dt:dtb`

The `dt:dtb` fill kind delivers the measured base DTB into a reserved region. As
with every [`fill`](core.md#fill), the action's `section` MUST be a Zero section:
it reserves the guest-physical range and holds no image bytes. The VMM populates
that range with a base DTB and measures the content.

```cbor-diag
{"type": "fill", "gpa": 0x2001000, "section": ".dtb", "kind": "dt:dtb"}
```

The bytes the VMM writes come from the bundled base named by the
[`dt:dtb`](#1-new-target-attribute-dtdtb) attribute, used as a non-authoritative
default the VMM MAY replace, or, in detached mode, from a base the VMM supplies
entirely. An image that needs an exact, non-substitutable base places it with a
[`default` load](core.md#load) instead: a `default` load writes the section's
bytes verbatim, so the host cannot substitute them.

A VMM MUST refuse to launch if the base it delivers exceeds the reserved section
size, or does not parse as a well-formed FDT ([Devicetree
Specification][devicetree] v0.4 or later).

The base is measured, per target:

- **`vm`**: ordinary guest memory (no measurement on this target).
- **`sev`**: `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_NORMAL` (measured into the
  launch digest).
- **`tdx`**: `KVM_TDX_INIT_MEM_REGION` with `KVM_TDX_MEASURE_MEMORY_REGION` set
  (`TDH.MEM.PAGE.ADD` then `TDH.MR.EXTEND` into MRTD).
- **`cca`**: `RMI_DATA_CREATE` (measured).

Because the host MAY substitute the base, the measurement records what the guest
actually received; see [Authorship and attestation
predictability](#authorship-and-attestation-predictability).

## 3. New `fill` kind: `dt:dtbo`

The `dt:dtbo` fill kind delivers a host-supplied devicetree overlay (DTBO), in
the format defined by the [Devicetree Specification][devicetree] v0.4 or later,
into a Zero section. The host selects the overlay content through VMM-defined
input, out of scope for PMI. The overlay is unmeasured: it does not contribute to
the target's launch measurement.

```cbor-diag
{"type": "fill", "gpa": 0x2011000, "section": ".dtbo", "kind": "dt:dtbo"}
```

The overlay allocates the resources the base leaves open: CPUs, memory, and NUMA
topology. Each of CPUs and memory is authored in full by exactly one party,
either the tenant in the measured base or the host in the overlay, never split.
A resource the base declares is fixed and measured, and the overlay MUST NOT
override it. An overlay is meaningless without a base to merge onto; if none is
present the merge fails, which is a denial of service.

Because the overlay is unmeasured, the guest MUST validate and merge it only from
memory the host cannot mutate after the check, that is, private memory. Two
placements satisfy this:

- **Unmeasured-private** (preferred where the target supports it): the VMM places
  the overlay in private, content-unmeasured guest memory. It is immutable after
  launch, so the guest validates it in place.
- **Shared** (fallback): the VMM places the overlay in shared memory; the guest
  MUST copy it, in a single pass, into private memory, then validate and merge
  the private copy.

Per target:

- **`vm`**: ordinary guest memory; no encryption, so the threat does not apply.
- **`sev`**: `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_UNMEASURED` (unmeasured-private).
- **`tdx`**: `KVM_TDX_INIT_MEM_REGION` with the measure flag clear (private,
  content unmeasured; the GPA still enters MRTD, which is deterministic).
- **`cca`**: no host-content unmeasured-private primitive exists, so the overlay
  is delivered in shared (NS) memory and the realm copies it into private memory
  before validating.

The overlay is adversarial input. The in-guest merger MUST validate it against an
allowlist that admits only host resource allocation (the CPUs, memory, and NUMA
the base leaves open) and merge it fail-closed, rejecting the launch on any
violation, before the guest relies on the platform description. The allowlist and
the merger's implementation are guest policy, out of scope for this spec.

## Authorship and attestation predictability

The base DTB is always measured, however it is delivered. Measurement records
what the guest received; it does not fix who chose the bytes. A substituted base
changes the launch measurement and is caught at attestation, but the measurement
is only *predictable*, and attestation only appraisable in advance, when the
base is **tenant-authored**. Detached mode exists to keep it so: it decouples DTB
distribution from PMI distribution (one image, many separately shipped tenant
DTBs) while the tenant remains the author. A VMM MAY author the base itself, but
the measurement then varies with host choice and cannot be appraised in advance;
images and deployments SHOULD keep the base tenant-authored.

## Examples

A `.pmi.vm` that loads a kernel, initrd, and command line, and bundles a base DTB
placed with an ordinary `default` load so its bytes are authoritative (attached).
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
