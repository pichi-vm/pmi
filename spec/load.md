# `load` action

The `load` action loads a PE section's bytes into guest memory. It is a
baseline action type reused across multiple PMI targets.

This document specifies the baseline schema and the default semantics.
Each target binding that references `load` is normative for how the action
behaves in that target — including the page-loading API used, the
measurement rules, and any additional or omitted schema fields. Where a
target binding and this document conflict, the target binding wins.

## Schema

```cddl
load = {
  "type"        => "load",
  "section"     => tstr,                ; PE section name to load
  ? "measured"  => bool,                ; default true
}
```

- **`section`** — the PE section whose bytes are loaded at the section's
  `VirtualAddress`. The VMM reads `VirtualAddress`, `SizeOfRawData`,
  `VirtualSize`, and `PointerToRawData` from the PE section header.

- **`measured`** — whether the loaded bytes are fed to the target's
  measurement API. Defaults to `true`. Setting to `false` suppresses
  measurement (e.g., for VMM-supplied data the verifier does not need to bind).

## Loading

The VMM loads pages from the lowest GPA to the highest within the section.
This ordering is significant: CC targets measure pages in submission order,
so lowest-to-highest produces a deterministic measurement.

There are three PE-section shapes:

1. **Data** (`SizeOfRawData > 0`, `VirtualSize == SizeOfRawData`). Load the
   on-disk data at `VirtualAddress`. The VMM chooses page granularity based on
   alignment — see [page granularity](pe.md#page-granularity).

2. **Padded** (`SizeOfRawData > 0`, `VirtualSize > SizeOfRawData`). Load the
   on-disk data at `VirtualAddress` as in case 1. Then zero-fill from
   `VirtualAddress + SizeOfRawData` to `VirtualAddress + VirtualSize`. The
   trailing zero region SHOULD use the target's zero-page API where
   available (e.g., `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_ZERO`), which
   validates pages as zero without transferring data. This is standard PE
   .bss-tail behavior — firmware or service modules that need reserved memory
   beyond their code use this to express it without file backing.

3. **Zero** (`SizeOfRawData == 0`, `VirtualSize > 0`). The entire region is
   zero-filled. No disk data is loaded. The VMM SHOULD use the target's
   zero-page API for the full range. This is how reserved memory regions are
   expressed.

## Measurement

When `measured` is true, the loaded bytes are fed to the target's
measurement API as part of loading. The distinction between on-disk data and
zero-fill matters: on-disk bytes are measured as normal data pages; zero-filled
bytes are measured as zero pages using the target's zero-page measurement
semantic, which may produce a different measurement than loading actual zeros
as data pages. VMM implementations MUST NOT substitute data-page loads for
zero-page operations or vice versa.
