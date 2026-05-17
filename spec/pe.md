# PE Constraints and Page Granularity

## PE Constraints

PMI imposes the following constraints on the PE:

- **The manifest MUST be stored in a `.pmi` PE section.** The section MUST be
  non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`).

- **Section names MUST fit in 8 bytes.** The PE `IMAGE_SECTION_HEADER.Name`
  field is a fixed 8-byte array. PMI does not use the COFF string table
  extension (which allows names longer than 8 bytes). Names shorter than 8 bytes
  are null-padded; names of exactly 8 bytes have no null terminator.

Tools which build PMI images MUST follow these constraints. Tools which consume
PMI images MAY reject images that do not conform.

## Page Granularity

PMI images MUST be built to support efficient loading with 2M huge pages. The
VMM allocates guest memory in 2M pages, then uses platform APIs (e.g.,
`SNP_LAUNCH_UPDATE`) to load data into those pages. Image authors control how
efficiently this loading happens through alignment. VMMs MAY always downgrade
to 4K page loading, but the image MUST NOT prevent 2M page loading where
possible.

There are two tiers of alignment, depending on section size:

### Large Sections (≥ 2M)

Sections like firmware (`.ovmf`), kernels (`.linux`), and initial ramdisks
(`.initrd`) are typically large. For these sections:

- `VirtualAddress` MUST be 2M-aligned.
- `PointerToRawData` MUST be 2M-aligned.
- `SizeOfRawData` MUST be a multiple of 2M.

This allows the VMM to mmap the file, pass each 2M chunk directly to the
platform API with no copy, and load it at a 2M-aligned GPA. The entire section
can be loaded in `SizeOfRawData / 2M` calls to the platform API.

### Small Sections (< 2M)

Sections like command lines (`.cmdline`), register state (`.sev.vms`), and
other single-page or small items:

- `VirtualAddress` MUST be 4K-aligned.
- `PointerToRawData` MUST be 4K-aligned.
- `SizeOfRawData` MUST be a multiple of 4K.
- Small sections SHOULD be packed contiguously within a 2M-aligned region.

The VMM allocates 2M pages in guest memory and loads small sections into them
at 4K granularity. Each small section requires a separate platform API call,
so packing them together minimizes the number of 2M boundaries they span and
reduces round-trips. The resulting guest still has 2M pages regardless of how
many 4K calls were needed to populate them.

## VirtualAddress sharing for mutually-exclusive sections

PE sections whose corresponding manifest segments carry disjoint `platforms`
annotations MAY share a `VirtualAddress`. The VMM loads at most one such segment
per launch (per the `platforms` filter), so there is never a runtime conflict in
guest memory.

Standard PE/UEFI loaders are not aware of PMI's platform filtering and may
handle overlapping sections in undefined ways (typically last-write-wins
during image load). Image authors using shared `VirtualAddress` for
PMI-only sections accept that the resulting PE may not behave correctly
when loaded by strict PE loaders outside the PMI consumption path.

## Manifest-authoritative loading

The manifest is authoritative for what the VMM does with each PE section. The
[`segments`](manifest/segments.md) array determines what the VMM loads into
guest memory or generates; the [`metadata`](manifest/metadata.md) array
determines what the VMM consumes for its own use without loading into
guest memory.

PE section flags such as `IMAGE_SCN_MEM_DISCARDABLE` govern only UEFI/PE
loader behavior — they signal to non-PMI loaders that a section should be
skipped or may be discarded after init. They do not affect the VMM's
loading or inspection decisions, which are driven entirely by the
manifest.
