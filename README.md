# PMI: Portable Machine Image

Traditionally, VMMs made all the decisions about how a guest boots: which
firmware to use, how memory is laid out, what CPUID values to expose. Guests
accept whatever the VMM provides. This works when the host is trusted.

Confidential Computing inverts this model. The tenant — not the host — decides
what software runs in the guest and how it is configured. The VMM becomes an
untrusted executor that must provide exactly what the guest image specifies,
nothing more. Hardware attestation verifies that it did.

IGVM was designed to address this for a specific case: loading paravisor
images into confidential VMs. Its design is elegant and well-suited for that
purpose.

However, the Linux ecosystem boots machines in many ways — direct kernel
loading, EFI stubs, firmware passthrough, service modules — across bare metal,
VMs, and confidential VMs. No single image format covers all of these
contexts. Each requires its own build pipeline and tooling.

PMI solves this by extending the PE format with one CBOR spec per platform
the image supports. Each spec lives in its own non-loaded PE section (by
convention `.pmi.<plat>` — e.g., `.pmi.vm`, `.pmi.sev`) and is a complete
launch recipe: an optional base DTB plus an ordered list of actions the VMM
performs. Systems that already boot from PE (UEFI, PXE, HTTP Boot,
systemd-boot) ignore the `.pmi.*` sections and boot as normal. VMMs targeting
a platform read that platform's section and execute its recipe.

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
