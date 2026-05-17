# Info

The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this
specification are to be interpreted as described in
[RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

The manifest's `info` array carries information the VMM consumes during launch
but does not load into guest memory. Each entry declares a `type` that
identifies what kind of information it carries; type-specific parameters
describe where to find the bytes (typically by naming a PE section).

Info is how the image hands the VMM information it needs to configure the guest
correctly — for example, the address-space layout the image expects (via a base
[DTB](dtb.md)). It is distinct from the [segments](segments.md) array, which
describes data the VMM loads into guest memory or generates for the guest to
consume.

## Schema

```cddl
info = {
  ? "platforms"  => { + tstr => any },  ; platform filter; absent = all
  "type"          => tstr,              ; info kind (e.g., "pmi:dtb")
  * tstr => any,                        ; type-specific parameters
}
```

- **`platforms`** — restricts the entry to the listed platforms. If present and
  the current platform is not a key in the map, the entry is skipped. If absent,
  the entry applies on every platform. Map values are reserved for future
  per-platform extensions; current PMI-defined types use `null`.

- **`type`** — identifies the info kind. See [Defined types](#defined-types) and
  [Extensibility](#extensibility).

## Extensibility

Every PMI-defined map accepts additional keys beyond those defined here. Type
values defined by this specification use the `"pmi:"` prefix (e.g.,
`"pmi:dtb"`). Extension types MUST use a collision-resistant namespaced form
with a non-`"pmi:"` prefix (e.g., `"vendor:custom"`). Consumers MUST ignore
unknown keys but MUST reject unknown type values.

## Processing

The VMM processes `info` entries before the `segments` array, so that
information learned (such as the address-space layout from a base DTB) is
available when allocating guest resources and loading segments.

For each declared info kind (each distinct `type`), the VMM picks the first
entry in array order whose `platforms` filter matches the current platform (or
which has no `platforms` field), and processes it according to its `type`.
Later matching entries with the same `type` are ignored. Image authors MUST
order platform-specific entries before any default entry, since a default entry
matches every platform and would otherwise win.

## Defined types

| Type        | Definition                              |
| ----------- | --------------------------------------- |
| `"pmi:dtb"` | Devicetree Blob — see [dtb.md](dtb.md). |
