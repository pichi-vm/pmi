# `fill`

The `fill` action populates a reserved GPA range at launch with
kind-specific content. The PE section MUST be a Zero section
(`SizeOfRawData == 0`, `VirtualSize > 0`) — it reserves the
address range but carries no on-disk data.

## Schema

```cddl
fill = {
  "type"    => "fill",
  "section" => tstr,                ; zero PE section to populate
  "kind"    => tstr,                ; REQUIRED; no default
}
```

The fill action MUST include a `kind` value; there is no default.
`fill` is extensible through its `kind` field, same as
[`load`](load.md).

## Kinds

PMI itself defines no `fill` kinds. Every `fill` kind is
namespaced — the kinds the active target itself adds use the
target name as the prefix (each target is a
[registered extension](../README.md#extensions)), and
any further kinds come from other registered or unregistered
extensions. The per-kind contract — what the VMM writes into the
region and whether the operation contributes to measurement —
lives in the spec that defines the kind.
