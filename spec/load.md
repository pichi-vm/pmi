# `load`

The `load` action loads a PE section's on-disk bytes into guest
memory at the section's `VirtualAddress`. The VMM reads
`VirtualAddress`, `SizeOfRawData`, `VirtualSize`, and
`PointerToRawData` from the PE section header.

## Schema

```cddl
load = {
  "type"    => "load",
  "section" => tstr,                ; PE section name to load
  ? "kind"  => tstr,                ; default "default"
}
```

`load` is extensible through its `kind` field. The only unprefixed
kind PMI itself defines is [`default`](#default-kind-default)
below. Every other kind is namespaced — including the kinds the
active target itself adds, since each target is a
[registered extension](../README.md#extensions) and uses its target
name as the prefix.

## Section shapes

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

## Default kind: `default`

The default kind for `load` is `default`. If `kind` is omitted,
`default` is assumed. Each target defines what `default` means for
that target's launch; the per-target chapter is authoritative. The
action's contract here is the wire format: declare `load` with a
section and an optional kind; the active target binds it to its
native ABI.
