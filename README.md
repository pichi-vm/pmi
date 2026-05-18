# PMI: Portable Machine Image

PMI is a standard interface for low-level virtual machine images. It solves
two specific problems.

**1. The platform-definition inversion.** Booting a machine has historically
followed a pattern where the firmware defines the platform layout — what
devices exist, where memory lives, what CPU configuration the guest sees — and
the guest software adapts to whatever the firmware presents. That made sense
on bare metal: the firmware ran first, had direct knowledge of the underlying
hardware, and the guest had limited capability to express what it required
(or to verify what it received) at early boot.

Virtual machines flip the capability asymmetry. A hypervisor has near-arbitrary
flexibility to compose any platform the guest will see, while the guest —
especially in early boot — still has very little ability to verify what the
platform actually is. Confidential Computing extends this into a security
boundary: the hypervisor is untrusted, and a platform definition the guest
cannot verify becomes a direct injection vector.

PMI inverts the model. The image declares the platform layout it requires
(via a base DTB describing devices, memory map, MMIO regions, interrupt
controllers, etc.); the VMM is obligated to provide exactly what is declared
or refuse to launch. Platform definition moves from the host to the image.

**2. The single-artifact problem.** Linux boot mechanisms vary widely: direct
kernel loading via the Linux boot protocol, EFI stub bundled with the kernel,
traditional firmware loading the kernel from disk, serviced confidential
computing with a privileged service module. Each historically requires its
own build pipeline and image format. PMI lets one PE binary cover all of
these — bare metal, virtual machine, confidential VM on multiple platforms —
via per-platform CBOR specs the image carries alongside its
kernel/firmware/etc.

A PMI image is a PE binary. For each launch path the image supports, it
carries a CBOR spec in its own non-loaded `.pmi.<plat>` PE section (e.g.,
`.pmi.vm`, `.pmi.sev`). Each spec is a complete launch recipe: an optional
base DTB plus an ordered list of actions the VMM performs. Systems that
already boot from PE (UEFI, PXE, HTTP Boot, systemd-boot) ignore the
`.pmi.*` sections and boot as normal. VMMs targeting a platform read that
platform's section and execute its recipe.

## Design Principles

1. **PE is the container.** UEFI, PXE, HTTP boot, systemd-boot, and VMMs all
   already consume PE. No new container format is needed. Standard PE tooling
   (mkosi, systemd-ukify, sbsign, objcopy) works on PMI images unmodified.

2. **One platform, one section, one recipe.** Each supported platform has its
   own self-contained CBOR spec in its own `.pmi.<plat>` PE section. Platforms
   are independent — they share conventions (the `dtb` field, the `load` and
   `dtbo` actions) but each fully specifies its own launch. No inheritance,
   no merging, no cross-platform filtering.

3. **The spec is declarative.** The VMM reads the active platform's spec and
   executes its `actions` in order. It does not introspect firmware binaries
   or rely on hardcoded conventions about image contents.

4. **Everything is a PE section.** Data loaded into guest memory,
   VMM-generated runtime data, platform-specific pages, launch-procedure
   inputs (policy, ID block), and the base DTB are all expressed as PE
   sections that the active platform spec references.

5. **Confidential Computing is additive.** A PMI image is a valid UKI that
   boots on bare metal and via standard direct/stubbed VM paths — UEFI ignores
   the `.pmi.*` sections. CC launch semantics layer on top via each CC
   platform's spec, never required.

6. **Strict, verifiable schemas.** Every action type, every key, every value
   the spec defines is exhaustive. A reference parser can decide a platform
   spec is valid or invalid with no third answer.

## Documentation

### Core

- [Why PMI?](spec/why.md) — Boot modes, format comparison, and why not IGVM
- [Overview](spec/overview.md) — Format overview, platforms, execution model
- [PE Constraints](spec/pe.md) — Alignment rules and page granularity
- [Examples](spec/examples.md) — Walkthroughs: `vm` + `sev`, serviced SVSM+OVMF

### Shared building blocks

- [DTB](spec/dtb.md) — Base DTB metadata and host-conformance contract
- [`load` action](spec/load.md) — Loading PE section bytes into guest memory
- [`dtbo` action](spec/dtbo.md) — Host-decided devicetree overlay

### Platforms

- [`vm`](spec/vm.md) — Non-CC virtual machines
- [`sev`](spec/sev.md) — AMD SEV 3.0 (SEV-SNP)
- [`tdx`](spec/tdx.md) — Intel TDX (TODO)
- [`cca`](spec/cca.md) — Arm CCA (TODO)
