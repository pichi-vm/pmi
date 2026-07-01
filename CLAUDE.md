# pmi

## Rules

- The `spec/` files are normative — the source of truth for the PMI wire format.
  The Rust types only mirror the per-target CBOR schemas defined there. Change
  the format spec-first (`spec:` commits), then update the types to match; never
  let the types redefine or silently drift from the spec.

<!-- Shared rules are in the .agent submodule. If .agent/ is empty:
     git submodule update --init --recursive -->
@.agent/karpathy/CLAUDE.md
@.agent/pichi.md
@.agent/rust.md
