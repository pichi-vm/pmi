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

`fill` kinds are defined per-target or by upper-layer extensions.
This action defines no kinds itself; each kind's contract — what
the VMM writes into the region and whether the operation
contributes to measurement — lives in the target chapter that
defines it (or the upper-layer spec that registers it). See the
per-target chapters and [Extensions](extensions.md) for the
namespacing rule.
