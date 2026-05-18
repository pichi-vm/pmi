# Base DTB

The per-platform manifest's `dtb` field names a PE section containing a
Devicetree Blob (FDT v17) that describes the image's expected platform
topology and address-space layout for this platform. The VMM reads the DTB
before processing segments and refuses to launch if it cannot conform.

The VMM reads this DTB during launch to learn:

- MMIO regions where the image expects virtual devices (PCIe controller,
  interrupt controller, virtio devices, console UART, timer, etc.)
- PCIe ECAM and BAR window addresses
- Reserved-memory regions to exclude from RAM allocation
- The platform topology the image was built against

Because the manifest is platform-specific (selected via the
[PMI index](../index.md)), there is no per-platform selection within the `dtb`
field — it names a single PE section. Different platforms with different
topologies can use different DTBs by having different manifests that name
different PE sections.

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

## Loading the DTB into guest memory

If the guest needs the DTB content in memory (for example, aarch64 Linux reads
the DTB via the `x0` register at boot, or an image's stub merges the base DTB
with the host overlay), the image author MUST also list the same PE section in
the `segments` array as a normal data segment. The `dtb` field and the
segment reference are independent: the `dtb` field causes VMM inspection; the
segment entry causes guest-memory loading.
