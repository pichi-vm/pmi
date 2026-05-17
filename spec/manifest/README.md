# Per-Platform Manifest

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

Each platform the image supports has its own CBOR-encoded **manifest** carried
in its own PE section. The [PMI index](../index.md) in `.pmi` maps platform
names to the PE sections containing these manifests.

A manifest is a complete recipe for launching the image on one specific
platform. There is no cross-platform filtering or selection within a manifest —
the index has already chosen the platform by selecting which manifest to read.

## Schema

```cddl
manifest = {
  "version"  => uint,                  ; schema version, currently 1
  ? "dtb"    => tstr,                  ; PE section containing the base DTB
  "segments" => [+ segment],           ; ordered launch recipe
  * tstr => any,                       ; extension point
}
```

- **`version`** — the manifest schema version. Currently `1`. VMMs MUST reject
  manifests with an unrecognized version.

- **`dtb`** — optional. Name of the PE section containing the base
  [DTB](dtb.md) describing the image's expected platform topology and
  address-space layout. The VMM reads this before processing segments and
  refuses to launch if it cannot conform to every declaration.

- **`segments`** — an ordered array of segment entries describing what the VMM
  should do at each step of the platform's launch procedure. See
  [segments.md](segments.md) for the segment schema and defined types.

All PMI-defined maps accept additional keys beyond those defined here.
Well-known keys are short, unnamespaced strings (e.g., `"section"`, `"type"`).
Extension keys MUST use a collision-resistant namespaced form
(`"namespace:key"`). Type values defined by this specification use the
`"pmi:"` prefix (e.g., `"pmi:load"`, `"pmi:dtbo"`, `"pmi:sev:vmsa"`);
extension types use a non-`"pmi:"` namespaced prefix. Consumers MUST ignore
unknown keys but MUST reject unknown type values.

## Platform Bindings

Each platform binding owns the set of segment types its manifest may use and
specifies how each type maps to the platform's launch API. Bindings are free
to define types for any launch step — initialization inputs, page loads,
finalization inputs.

- [Native](platforms/native.md) — non-CC virtual machines
- [AMD SEV 3.0](platforms/sev.md) — typed launch inputs, page loads,
  ID-block-based attestation
- [Intel TDX](platforms/tdx.md) — TODO
- [Arm CCA](platforms/cca.md) — TODO
