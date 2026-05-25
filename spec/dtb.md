# `dtb` Extension

**Prefix:** `dtb`.

The `dtb` extension provides the cross-target mechanism for delivering platform
description (memory map, MMIO/IO regions, CPU topology) to the guest. It defines
one extension point:

1. The new `fill` kind [`dtb:fdt`](#1-new-fill-kind-dtbfdt).

For why platform description is delivered as a devicetree, see
[Motivation §2](motivation.md#2-portable-safe-platform-definition-and-attestation).

## 1. New `fill` kind: `dtb:fdt`

The VMM places a host-supplied flattened devicetree blob (DTB), in the format
defined by the [Devicetree Specification][devicetree] v0.4 or later, into guest
memory at the section's GPA.

The host selects the DTB content via VMM-defined input, out of scope for PMI.
The referenced PE section MUST be a Zero section (`SizeOfRawData == 0`) with
`VirtualSize` large enough to hold the DTB. The DTB is unmeasured — it does not
contribute to the target's launch measurement.

Because the DTB is unmeasured and host-supplied, the guest MUST validate it
before relying on it; the validation policy is the guest's and is out of scope
for this spec.

[devicetree]: https://www.devicetree.org/specifications/
