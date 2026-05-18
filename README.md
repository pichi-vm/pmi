# PMI: Portable Machine Image

Traditionally, VMMs made all the decisions about how a guest boots: which
firmware to use, how memory is laid out, what CPUID values to expose. Guests
accept whatever the VMM provides. This works when the host is trusted.

Confidential Computing inverts this model. The tenant — not the host — decides
what software runs in the guest and how it is configured. The VMM becomes an
untrusted executor that must provide exactly what the guest image specifies,
nothing more. Hardware attestation verifies that it did.

IGVM was designed to address this for a specific case: loading paravisor images
into confidential VMs. Its design is elegant and well-suited for that purpose.

However, the Linux ecosystem boots machines in many ways — direct kernel
loading, EFI stubs, firmware passthrough, service modules — across bare metal,
VMs, and confidential VMs. No single image format covers all of these contexts.
Each requires its own build pipeline and tooling.

PMI solves this by extending the PE format with two CBOR layers: a small
**index** in `.pmi` that names the platforms the image supports, and a
**per-platform manifest** in another PE section (by convention `.pmi.<plat>`)
that carries the complete launch recipe for that one platform. Systems that
already boot from PE (UEFI, PXE, HTTP Boot, systemd-boot) ignore both and boot
as normal. VMMs that understand PMI read the index, pick the platform they
target, and execute that manifest's recipe. The same artifact boots on bare
metal, in a VM, and in a confidential VM on multiple platforms.

## Design Principles

1. **PE is the container.** UEFI, PXE, HTTP boot, systemd-boot, and VMMs all
   already consume PE. No new container format is needed. Standard PE tooling
   (mkosi, systemd-ukify, sbsign, objcopy) works on PMI images unmodified.

2. **The manifest is declarative.** The VMM reads the active per-platform
   manifest and executes it. It does not introspect firmware binaries, rely
   on hardcoded conventions, or make assumptions about image contents.
   Everything the VMM needs to know is in the manifest.

3. **One manifest per platform.** Each supported platform has its own complete
   recipe in its own PE section. No cross-platform filtering, no merging.
   The `.pmi` index resolves the target platform to its manifest section, and
   the rest is platform-local.

4. **Everything is a PE section.** Data loaded into guest memory,
   VMM-generated runtime data, platform-specific pages, launch-procedure
   inputs (policy, ID block, etc.), and the base DTB are all expressed as PE
   sections that the active manifest references. The manifest expresses
   regions and types, not pages; the host decides page granularity.

5. **Confidential Computing is additive.** A PMI image without an index is
   still a valid UKI that boots on bare metal and via standard direct/stubbed
   VM paths — UEFI ignores `.pmi*`. CC semantics layer on top via per-platform
   manifests, never required.

6. **Strict, verifiable schemas.** Every type, every key, every value the
   spec defines is exhaustive. A reference parser can decide a manifest is
   valid or invalid with no third answer.

## Documentation

- [Why PMI?](spec/why.md) — Boot modes, format comparison, and why not IGVM
- [Overview](spec/overview.md) — Format overview, execution model, measurement
- [PE Constraints](spec/pe.md) — Alignment rules and page granularity
- [PMI Index](spec/index.md) — The `.pmi` section: platform discovery
- [Examples](spec/examples.md) — Walkthroughs: vm + SEV, serviced SVSM+OVMF

### Per-Platform Manifest

- [Manifest](spec/manifest/README.md) — Top-level schema and versioning
- [Segments](spec/manifest/segments.md) — Segment schema, defined types,
  loading rules, measurement
- [DTB](spec/manifest/dtb.md) — Base DTB and host-conformance contract

### Platform Bindings

- [VM](spec/manifest/platforms/vm.md) — `pmi:vm:vcpu` (non-CC VMs)
- [AMD SEV 3.0](spec/manifest/platforms/sev.md) — Launch-input and page-load
  segment types, ID-block-based attestation
- [Intel TDX](spec/manifest/platforms/tdx.md) — TODO
- [Arm CCA](spec/manifest/platforms/cca.md) — TODO
