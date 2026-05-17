# Metadata

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

The manifest's `metadata` array carries information the VMM consumes during
launch but does not load into guest memory. Each entry references a PE
section by name and identifies what kind of metadata the section contains.

Metadata is how the image hands the VMM information it needs to configure
the guest correctly — for example, the address-space layout the image
expects (via a base [DTB](dtb.md)). It is distinct from the
[segments](segments.md) array, which describes data the VMM loads into
guest memory or generates for the guest to consume.

## Schema

```cddl
metadata = {
  "section"      => tstr,                ; PE section name
  "type"         => tstr,                ; metadata kind (e.g., "pmi:dtb")
  ? "platforms"  => { + tstr => any },   ; platform filter
  * tstr => any,                         ; extension point
}
```

## Extensibility

Every PMI-defined map accepts additional keys beyond those defined here.
Type values defined by this specification use the `"pmi:"` prefix (e.g.,
`"pmi:dtb"`). Extension types MUST use a collision-resistant namespaced form
with a non-`"pmi:"` prefix (e.g., `"vendor:custom"`). VMMs MUST reject types
they do not recognize.

## Processing

For each metadata entry whose `platforms` filter matches the current platform
(or which has no `platforms` field), the VMM looks up the PE section
named by `section` in the PE section table, reads its on-disk bytes
(`SizeOfRawData` bytes at `PointerToRawData`), and processes them according to
the `type` field.

The VMM MUST process all metadata entries before processing the `segments`
array, so that information learned from metadata (such as the
address-space layout from a base DTB) is available when allocating guest
resources and loading segments.

## Platform filter

The `platforms` field has the same semantics as on
[segments](segments.md#schema): if the current platform is not a key in the
map, the entry is skipped. Multiple metadata entries with the same `type` and
disjoint `platforms` filters are valid; the VMM uses the entry whose filter
matches the current platform.

If `platforms` is absent, the entry applies on every platform.

## Composition with segments

The same PE section MAY be referenced by both a `metadata` entry and a
`segments` entry. The metadata entry causes VMM-side inspection; the
segments entry causes the bytes to be loaded into guest memory at the PE
section's `VirtualAddress`. The two references are independent.

## Defined types

| Type        | Definition                              |
| ----------- | --------------------------------------- |
| `"pmi:dtb"` | Devicetree Blob — see [dtb.md](dtb.md). |
