# Page Granularity

VMMs often have to choose a page granularity for operations, including the
`load` and `fill` actions in this specification. It is highly efficient to be
able to `mmap()` the PMI file on the disk and then pass the memory directly to
the hypervisor or firmware. This permits zero-copy construction. However, this
can only be done if the pages are correctly aligned on disk.

If a VMM has to support unaligned pages, then it must build page copy semantics
into its loader, which is costly and error prone. However, a 2M alignment is
always compatible with a 4K alignment. This means that VMMs can efficiently
downgrade from a larger alignment on disk to a smaller alignment requirement in
memory.

Therefore, to facilitate efficient VMM construction, PMI makes the following
alignment rules.

## Large Sections (≥ 2M)

Sections referenced in the `load` and `fill` actions whose `VirtualSize` is ≥ 2M
have the following alignment requirements:

- `VirtualAddress` MUST be 2M-aligned.
- `PointerToRawData` MUST be 2M-aligned.
- `SizeOfRawData` MUST be a multiple of 2M.

This allows the VMM to mmap the file, pass each 2M chunk directly to the target
API with no copy, and load it at a 2M-aligned GPA. The entire section can be
loaded in `SizeOfRawData / 2M` calls to the target API.

## Small Sections (< 2M)

Sections referenced in the `load` and `fill` actions whose `VirtualSize` is < 2M
have the following alignment requirements:

- `VirtualAddress` MUST be 4K-aligned.
- `PointerToRawData` MUST be 4K-aligned.
- `SizeOfRawData` MUST be a multiple of 4K.
- Small sections SHOULD be packed contiguously within a 2M-aligned region.

The VMM allocates 2M pages in guest memory and loads small sections into them at
4K granularity. On CC targets each small section typically requires a separate
target API call, so packing them together minimizes the number of 2M boundaries
they span and reduces round-trips. The resulting guest still has 2M pages
regardless of how many 4K calls were needed to populate them.
