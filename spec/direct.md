# `direct` Extension

**Prefix:** `direct`.

The `direct` extension delivers the guest's platform description (memory map,
MMIO/IO regions, CPU topology) as a complete, host-supplied flattened devicetree
blob. It defines one extension point:

1. The new `fill` kind [`direct:dtb`](#1-new-fill-kind-directdtb).

The host supplies the entire DTB at launch; the guest validates it post-hoc. The
DTB does not contribute to the launch measurement. See
[Motivation §2](motivation.md#splitting-platform-definition-from-resource-allocation)
for the trust model — including the protection `direct` cannot reach on the
image-owned half of a DTB, and the [`merged`](merged.md) extension that does.

## 1. New `fill` kind: `direct:dtb`

The VMM fills the section with a host-supplied flattened devicetree blob (DTB),
in the format defined by the [Devicetree Specification][devicetree] v0.4 or
later. The host selects the DTB content via VMM-defined input, out of scope for
PMI. The DTB is **unmeasured** — it does not contribute to the target's launch
measurement — so the guest MUST validate it before relying on it; the validation
policy is the guest's and is out of scope for this spec.

The DTB MUST advertise every populated region as guest RAM. Define the guest's
usable RAM as the union of `reg` entries on nodes with `device_type = "memory"`
whose `status` is absent or `"okay"`, minus any range covered by a
`/reserved-memory` child carrying the `no-map` property. For each `load` and
`fill` action in the active target's `actions`, every byte of the referenced PE
section's `[VirtualAddress, VirtualAddress + VirtualSize)` range MUST lie within
the guest's usable RAM. The section receiving the DTB itself is no exception. A
VMM MUST refuse to launch on a DTB that does not satisfy this requirement.

[devicetree]: https://www.devicetree.org/specifications/

## Example

A `.pmi.vm` that loads a kernel, initrd, and command line, and fills `.dtb` with
a host-supplied DTB:

```cbor-diag
{
  "version": 1,
  "vm:vcpu": {"rip": 0x100000, "rsp": 0x80000, "rflags": 0x2},
  "actions": [
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"},
    {"type": "fill", "section": ".dtb", "kind": "direct:dtb"}
  ]
}
```
