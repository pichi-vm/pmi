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

PMI solves this by extending the PE format with a CBOR-encoded manifest — the
complete recipe for launching a guest. Systems that already boot from PE (UEFI,
PXE, HTTP Boot, systemd-boot) ignore the manifest and boot as normal. VMMs that
understand PMI read the manifest and execute its instructions. The same artifact
boots on bare metal, in a VM, and in a confidential VM on multiple platforms.

## Design Principles

1. **PE is the container.** UEFI, PXE, HTTP boot, systemd-boot, and VMMs all
   already consume PE. No new container format is needed. Standard PE tooling
   (mkosi, systemd-ukify, sbsign, objcopy) works on PMI images unmodified.

2. **The manifest is declarative.** The VMM reads the manifest and executes it.
   It does not introspect firmware binaries, rely on hardcoded conventions, or
   make assumptions about image contents. Everything the VMM needs to know is in
   the manifest.

3. **Everything is a PE section.** Data loaded from the PE, VMM-generated
   runtime data, platform-specific pages, and VMM-inspectable image data are
   all expressed as PE sections, declared in the manifest's `segments` and
   `info` arrays. Each entry carries a type that selects its behavior and a
   platforms filter that selects where it applies. The manifest expresses
   regions, not pages; the host decides page granularity.

4. **Policy is separate and mergeable.** The image may embed required platform
   policy. A deployer may supply external policy that is deep-merged with the
   image policy, with the image taking precedence on conflicts. Policy is not
   measured — it appears in the attestation report for remote verification.

5. **Confidential Computing is additive.** A PMI without a manifest can be a
   traditional UKI. A PMI with a manifest boots identically on non-CC VMMs that
   ignore the manifest. CC semantics are layered on top, never required.

6. **Extensible everywhere.** Every structure accepts unknown keys. New
   platforms, segment types, and policy fields require no schema changes. This
   implies that VMMs can provide VMM-specific extensions without breaking
   compatibility with other VMMs.

## Documentation

- [Why PMI?](spec/why.md) — Boot modes, format comparison, and why not IGVM
- [Overview](spec/overview.md) — Format overview, execution model, measurement
- [PE Constraints](spec/pe.md) — Alignment rules and page granularity
- [Examples](spec/examples.md) — Walkthroughs: direct boot, SVSM+OVMF,
  per-platform DTBs

### Manifest

- [Manifest](spec/manifest/README.md) — Top-level schema, extensibility,
  versioning
- [Segments](spec/manifest/segments.md) — Segment schema, loading, segment
  types, platforms filter
- [Info](spec/manifest/info.md) — VMM-inspectable image data
- [DTB](spec/manifest/dtb.md) — the `pmi:dtb` info type
- [Policy](spec/manifest/policy.md) — Policy schema and merge algorithm

### Platform Bindings

- [AMD SEV 3.0](spec/manifest/platforms/sev.md) — Policy, segment types, API
  mapping
- [Intel TDX](spec/manifest/platforms/tdx.md) — TODO
- [Arm CCA](spec/manifest/platforms/cca.md) — TODO
- [Native](spec/manifest/platforms/native.md) — No CC
