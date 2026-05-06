# Manifest

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

The `.pmi` PE section contains a CBOR-encoded manifest — the complete
instructions for how a VMM should launch a guest from this image.

The manifest serves three purposes:

1. **Section loading.** It tells the VMM which PE sections to load into guest
   memory, in what order, and how each section should be treated — loaded from
   disk, filled with VMM-generated data, or handled by a platform-specific API.

2. **Platform targeting.** It allows a single image to contain sections for
   multiple platforms (e.g., SEV, TDX, native). The VMM selects the relevant
   platform and skips sections that do not apply.

3. **Policy.** It carries platform launch policy that the VMM merges with any
   deployer-supplied policy before initializing the confidential computing
   platform. The image author sets the security floor; the deployer fills in the
   rest.

## Schema

```cddl
manifest = {
  "version"      => uint,              ; schema version, currently 1
  "sections"     => [+ section]        ; see sections.md
  ? "policy"      => policy            ; see policy.md
  * tstr => any,                       ; extension point
}
```

- **`version`** — the manifest schema version. Currently `1`. VMMs MUST reject
  manifests with an unrecognized version.

- **`sections`** — an ordered array of section entries. See
  [sections.md](sections.md) for the section schema, loading rules, fill types,
  and platform annotations.

- **`policy`** — an optional map of platform launch policies. See
  [policy.md](policy.md) for the policy schema, merge algorithm, and
  per-platform definitions.

All PMI-defined maps accept additional keys beyond those defined here.
Well-known keys are short, unnamespaced strings (e.g., `"name"`, `"measured"`,
`"sev"`). Extension keys MUST use a collision-resistant namespaced form:
`"namespace:key"` (e.g., `"vendor:feature"`). Consumers MUST ignore keys they do
not recognize.

## Platform Bindings

Each CC platform defines its own policy schema and section annotation values.
These are specified in separate binding documents:

- [AMD SEV 3.0](platforms/sev.md) — Policy, annotations, API mapping
- [Intel TDX](platforms/tdx.md) — TODO
- [Arm CCA](platforms/cca.md) — TODO
- [Native](platforms/native.md) — No CC
