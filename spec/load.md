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
  ? "kind"  => tstr,                ; default "measured"
}
```

`load` is extensible through its `kind` field. The field accepts
any of the unprefixed kinds the active target defines, plus any
namespaced kind a registered or unregistered extension defines.

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

## Default kind: `measured`

The default kind for `load` is `measured`. If `kind` is omitted,
`measured` is assumed. Each target defines what `measured` means
for that target's launch — typically "perform the target's
standard measured-load operation against the firmware ABI." On
the non-CC `vm` target `measured` reduces to placing the bytes in
guest memory with no measurement step (vm has none).

The per-target chapter is authoritative for the exact firmware
sequence each kind drives. The action's contract here is the wire
format: declare `load` with a section and an optional kind; the
active target binds it to its native ABI.
