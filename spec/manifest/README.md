# Manifest

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

The `.pmi` PE section contains a CBOR-encoded manifest — the complete
instructions for how a VMM should launch a guest from this image.

The manifest serves four purposes:

1. **Segment loading.** It tells the VMM which PE sections to load into guest
   memory, in what order, and how each segment should be treated — loaded from
   disk, filled with VMM-generated data, or handled by a platform-specific API.

2. **DTB inspection.** It points the VMM at the base [DTB](dtb.md) describing
   the image's expected platform topology and address-space layout. The VMM
   reads the DTB to learn what hardware it must provide; it MUST refuse to
   launch if it cannot match every declaration. The DTB is consumed by the VMM
   but not loaded into guest memory by this reference.

3. **Platform targeting.** It allows a single image to contain segments for
   multiple platforms (e.g., SEV, TDX, native). The VMM selects the relevant
   platform and skips segments that do not apply.

4. **Policy.** It carries platform launch policy that the VMM merges with any
   deployer-supplied policy before initializing the confidential computing
   platform. The image author sets the security floor; the deployer fills in the
   rest.

## Schema

```cddl
manifest = {
  "version"   => uint,                 ; schema version, currently 1
  ? "dtb"     => [+ dtb-ref],         ; see dtb.md
  "segments"  => [+ segment],          ; see segments.md
  ? "policy"  => policy,               ; see policy.md
  * tstr => any,                       ; extension point
}
```

- **`version`** — the manifest schema version. Currently `1`. VMMs MUST reject
  manifests with an unrecognized version.

- **`dtb`** — an optional ordered array of base DTB references. The VMM picks
  the first entry whose `platforms` filter matches and reads the referenced
  FDT before processing segments. See [dtb.md](dtb.md) for the schema,
  selection rule, format, and host-conformance contract.

- **`segments`** — an ordered array of segment entries. See
  [segments.md](segments.md) for the segment schema, loading rules, defined
  segment types, and platforms filter.

- **`policy`** — an optional map of platform launch policies. See
  [policy.md](policy.md) for the policy schema, merge algorithm, and
  per-platform definitions.

All PMI-defined maps accept additional keys beyond those defined here.
Well-known keys are short, unnamespaced strings (e.g., `"section"`, `"type"`,
`"platforms"`). Extension keys MUST use a collision-resistant namespaced form:
`"namespace:key"` (e.g., `"vendor:feature"`). Type values defined by this
specification use the `"pmi:"` prefix (e.g., `"pmi:load"`, `"pmi:dtb"`,
`"pmi:dtbo"`, `"pmi:sev:vmsa"`); extension types use a non-`"pmi:"` namespaced
prefix. Consumers MUST ignore unknown keys but MUST reject unknown type values.

## Platform Bindings

Each CC platform defines its own policy schema and segment types. These are
specified in separate binding documents:

- [AMD SEV 3.0](platforms/sev.md) — Policy, segment types, API mapping
- [Intel TDX](platforms/tdx.md) — TODO
- [Arm CCA](platforms/cca.md) — TODO
- [Native](platforms/native.md) — No CC
