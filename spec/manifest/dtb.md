# Base DTB

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

The manifest's `dtb` array references one or more PE sections, each containing
a Devicetree Blob (FDT v17) that describes the image's expected platform
topology and address-space layout. The VMM reads the selected DTB during launch
to learn:

- MMIO regions where the image expects virtual devices (PCIe controller,
  interrupt controller, virtio devices, console UART, timer, etc.)
- PCIe ECAM and BAR window addresses
- Reserved-memory regions to exclude from RAM allocation
- The platform topology the image was built against

## Schema

```cddl
dtb-ref = {
  ? "platforms" => [+ tstr],            ; platform filter; absent = all
  "section"     => tstr,                ; PE section containing the FDT
  * tstr => any,                        ; extension point
}
```

- **`platforms`** — restricts the entry to the listed platforms. If present and
  the current platform is not in the list, the entry is skipped. If absent,
  the entry applies on every platform.

- **`section`** — name of the PE section containing the FDT.

## Selection

The VMM picks the first entry in array order whose `platforms` filter matches
the current platform (or which has no `platforms` field). Later matching
entries are ignored. Image authors MUST order platform-specific entries before
any default entry, since a default entry matches every platform and would
otherwise win.

The VMM MUST process the selected DTB before the `segments` array, so that
information learned from it (such as the address-space layout) is available
when allocating guest resources and loading segments.

## Format

The PE section's bytes MUST be a valid Flattened Devicetree binary conforming
to the Devicetree Specification:

- Header magic `0xd00dfeed`
- `last_comp_version` ≤ 17 ≤ `version`
- `totalsize` ≤ PE section `SizeOfRawData`
- All referenced offsets within their respective blocks (memory reservation
  block, structure block, strings block)

The VMM MUST reject a DTB that fails any of these checks.

## VMM conformance

This is the core host-conformance contract for PMI: **the image declares what it
requires; the host conforms or refuses.**

The VMM MUST validate that it can provide every hardware capability the base DTB
declares. If any declaration cannot be satisfied — a device the VMM cannot
expose, an interrupt controller version the host does not support, an MMIO
region the host cannot allocate at the requested GPA, a PCIe configuration the
host cannot match, or any other declared resource the host cannot supply — the
VMM MUST fail the launch with a clear indication of which declaration was
unsupported.

The VMM MUST NOT silently substitute a different configuration for a declared
one, omit declared hardware, or relocate resources to addresses other than those
declared.

## Image-side responsibilities

The image bakes the DTB describing its platform topology. The DTB SHOULD omit
`/memory`, `/cpus`, and `/distance-map` nodes; the host fills these through a
separate [`pmi:dtbo`](segments.md) overlay, since memory size, vCPU count, and
NUMA topology are host-decided.

The DTB MAY include any other nodes the image needs to declare: interrupt
controller, timer, console, virtio devices, PCIe controller, `/chosen` (cmdline,
initrd pointers), reserved-memory regions, etc. The image author owns the
address-space layout for everything outside the three host-fillable paths above;
the VMM must place its emulated devices at the image-declared addresses or
refuse to launch.

Nodes the image declares MAY be annotated with `numa-node-id` by the host's
overlay (see [`pmi:dtbo`](segments.md)). This is the only property the host may
add to non-`/cpus` / non-`/memory@*` / non-`/distance-map` nodes. The image does
NOT pre-populate `numa-node-id` on its own declared nodes; the host supplies
these at launch since the NUMA topology of the deployment host is not knowable
at image build time.

## Per-platform variants

When per-platform DTB sections share a `VirtualAddress` (per the
[VirtualAddress sharing rule](../pe.md#virtualaddress-sharing-for-mutually-exclusive-sections)),
they MAY also share PE section names; the `platforms` filter on the `dtb`
entries resolves which one is selected.

## Loading the DTB into guest memory

If the guest needs the DTB content in memory (for example, aarch64 Linux reads
the DTB via the `x0` register at boot, or an image's stub merges the base DTB
with the host overlay), the image author MUST also list the same PE section in
the `segments` array as a normal data segment. The `dtb` reference and the
segment reference are independent: the `dtb` entry causes VMM inspection; the
segment entry causes guest-memory loading.
