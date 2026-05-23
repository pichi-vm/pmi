# Actions

An `action` is a single step in a target spec's `actions` array.
Actions are the building blocks of a launch recipe: each one
selects an operation by `type` and parameterises it with per-type
fields.

This document defines the actions PMI itself ships. Upper layers
MAY register additional action types; see
[Extensions](extensions.md).

## Common shape

Every action is a CBOR map:

```cddl
action = {
  "type" => tstr,                          ; selects the operation
  ; per-type fields
}
```

`type` is the only universal action field. Everything else is
defined per action type. The two action types PMI defines today
are [`load`](#load) and [`fill`](#fill).

## `load`

The `load` action loads a PE section's on-disk bytes into guest
memory at the section's `VirtualAddress`. The VMM reads
`VirtualAddress`, `SizeOfRawData`, `VirtualSize`, and
`PointerToRawData` from the PE section header.

### Schema

```cddl
load = {
  "type"    => "load",
  "section" => tstr,                ; PE section name to load
  ? "kind"  => tstr,                ; default "measured"
}
```

`load` is extensible through its `kind` field. The field accepts
any of the unprefixed kinds PMI or the active target defines, plus
any namespaced kind a registered or unregistered extension
defines.

### Section shapes

There are three PE-section shapes:

1. **Data** (`SizeOfRawData > 0`, `VirtualSize == SizeOfRawData`).
   Load the on-disk data at `VirtualAddress`. The VMM chooses page
   granularity based on alignment — see
   [page granularity](pe.md#page-granularity).
2. **Padded** (`SizeOfRawData > 0`, `VirtualSize > SizeOfRawData`).
   Load the on-disk data at `VirtualAddress` as in the Data shape
   above. Then zero-fill from `VirtualAddress + SizeOfRawData` to
   `VirtualAddress + VirtualSize`. This is standard PE `.bss`-tail
   behavior — firmware or service modules that need reserved
   memory beyond their code use this to express it without file
   backing.
3. **Zero** (`SizeOfRawData == 0`, `VirtualSize > 0`). The entire
   region is zero-filled. No disk data is loaded. This is how
   reserved memory regions are expressed.

### Default kind: `measured`

The default kind for `load` is `measured`. If `kind` is omitted,
`measured` is assumed. Each target defines what `measured` means
for that target's launch — typically "perform the target's
standard measured-load operation against the firmware ABI." On
the non-CC `vm` target `measured` reduces to placing the bytes in
guest memory with no measurement step (vm has none).

The per-target chapter is authoritative for the exact firmware
sequence each kind drives. PMI's contract here is the wire format:
declare `load` with a section and an optional kind; the active
target binds it to its native ABI.

## `fill`

The `fill` action populates a reserved GPA range at launch with
kind-specific content. The PE section MUST be a Zero section
(`SizeOfRawData == 0`, `VirtualSize > 0`) — it reserves the
address range but carries no on-disk data.

### Schema

```cddl
fill = {
  "type"    => "fill",
  "section" => tstr,                ; zero PE section to populate
  "kind"    => tstr,                ; REQUIRED; no default
}
```

The fill action MUST include a `kind` value; there is no default.
`fill` is extensible through its `kind` field, same as `load`.

### Kinds

`fill` kinds are defined per-target or by upper-layer extensions.
PMI itself defines no `fill` kinds at the action layer; each
kind's contract — what the VMM writes into the region and whether
the operation contributes to measurement — lives in the target
chapter that defines it (or the upper-layer spec that registers
it). See the per-target chapters and
[Extensions](extensions.md) for the namespacing rule.
