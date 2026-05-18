# Base DTB

The `dtb` field in a target spec names a PE section containing the base DTB
the VMM inspects before launch. See [Overview](overview.md#solving-the-platform-definition-inversion)
for the conceptual role of the DTB.

This document is the normative reference for the format, the host
conformance contract, and image-side authoring rules.

## Format

The PE section's bytes MUST be a valid Flattened Devicetree binary
conforming to the Devicetree Specification:

- Header magic `0xd00dfeed`
- `last_comp_version` ≤ 17 ≤ `version`
- `totalsize` ≤ PE section `SizeOfRawData`
- All referenced offsets within their respective blocks (memory reservation
  block, structure block, strings block)

The VMM MUST reject a DTB that fails any of these checks.

## Host conformance

The VMM MUST validate that it can provide every hardware capability the
base DTB declares. If any declaration cannot be satisfied — a device the
VMM cannot expose, an interrupt controller version the host does not
support, an MMIO region the host cannot allocate at the requested GPA, a
PCIe configuration the host cannot match, or any other declared resource
the host cannot supply — the VMM MUST fail the launch with a clear
indication of which declaration was unsupported.

The VMM MUST NOT silently substitute a different configuration for a
declared one, omit declared hardware, or relocate resources to addresses
other than those declared.

## Image-side responsibilities

The DTB SHOULD omit `/memory`, `/cpus`, and `/distance-map` nodes; the host
fills these through a separate [`dtbo`](vm.md#dtbo-action) overlay, since memory
size, vCPU count, and NUMA topology are host-decided.

The DTB MAY include any other nodes the image needs to declare: interrupt
controller, timer, console, virtio devices, PCIe controller, `/chosen`
(cmdline, initrd pointers), reserved-memory regions, etc. The image author
owns the address-space layout for everything outside the three
host-fillable paths above.

Nodes the image declares MAY be annotated with `numa-node-id` by the
host's overlay (see [`dtbo`](vm.md#dtbo-action)). This is the only property the host
may add to non-`/cpus` / non-`/memory@*` / non-`/distance-map` nodes. The
image MUST NOT pre-populate `numa-node-id` on its own declared nodes; the
host supplies these at launch since the NUMA topology of the deployment
host is not knowable at image build time.

## Loading the DTB into guest memory

If the guest needs the DTB content in memory (for example, aarch64 Linux
reads the DTB via the `x0` register at boot, or an image's stub merges the
base DTB with the host overlay), the image author MUST also list the same
PE section as a `load` action in the target's actions array. The `dtb`
field and the `load` action are independent: the `dtb` field causes VMM
inspection; the `load` action causes guest-memory loading.
