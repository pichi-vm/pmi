# Page Granularity

VMMs often have to choose a page granularity for operations, including the
`load` and `fill` actions in this specification. Placing a section's bytes in
guest memory always involves a copy, performed by the target firmware on
confidential targets (which copies a source page into encrypted guest memory), or
by the VMM otherwise. These alignment rules do not eliminate that copy; they make
it efficient, letting a VMM memory-map the PMI file (e.g., via POSIX `mmap()`) at
aligned offsets and hand whole aligned chunks to the copy without additional
staging. This requires the section to be correctly aligned on disk.

A 2M alignment is always compatible with a 4K alignment, so a VMM can efficiently
downgrade from a larger alignment on disk to a smaller alignment requirement in
memory. These rules govern only file and guest mapping; the loading page size a
VMM chooses never enters a launch measurement (see [Measurement
determinism](core.md#measurement-determinism)).

Therefore, to facilitate efficient VMM construction, PMI makes the following
alignment rules. They constrain only the *start* of a section, its `gpa` and
`PointerToRawData`. A section's `SizeOfRawData` is not otherwise constrained (it
follows PE's own `FileAlignment`); a section occupies whole 4 KiB guest pages,
and any bytes in its final page beyond the section's data are zero.

## Large Sections (≥ 2M)

Sections referenced in the `load` and `fill` actions whose `VirtualSize` is ≥ 2M
have the following alignment requirements:

- `gpa` MUST be 2M-aligned.
- `PointerToRawData` MUST be 2M-aligned.

Because the section's start is 2M-aligned in both the file and guest memory, the
VMM can mmap the file and hand each whole 2M chunk to the copy at a 2M-aligned
GPA. A large section is loaded as `floor(SizeOfRawData / 2M)` such 2M chunks,
plus (when `SizeOfRawData` is not a multiple of 2M) a trailing tail (< 2M)
loaded at 4K granularity exactly as a small section. Only the section's *start*
is 2M-aligned; its size is not padded up to a 2M multiple, so a 2.1 MiB section
occupies ~2.1 MiB on disk rather than 4 MiB, and the remainder of its final 2M
region MAY be used to pack small sections (see below).

## Small Sections (< 2M)

Sections referenced in the `load` and `fill` actions whose `VirtualSize` is < 2M
have the following alignment requirements:

- `gpa` MUST be 4K-aligned.
- `PointerToRawData` MUST be 4K-aligned.

A VMM MUST correctly load a small section wherever it is placed, at 4K
granularity. Packing is never required, and images that do not pack are fully
conformant.

As an optional layout optimization, an image author MAY pack small sections
contiguously within a 2M-aligned region (including the freed tail of a large
section, above). The benefit is target-specific: on SEV a VMM MAY then submit a
densely-filled 2M-aligned region as a single 2M `SNP_LAUNCH_UPDATE` (fewer PSP
round-trips; the launch digest is unchanged, being computed per 4K), and any
target MAY back the dense region with a 2M guest page. On TDX and CCA the
measured-load primitive is 4K-granular, so packing does not reduce the number of
target API calls.
