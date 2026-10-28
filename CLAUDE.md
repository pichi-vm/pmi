# pmi

## Rules

- Modular `spec/*.md` files are the editable normative source for the PMI wire
  format; `spec/pmi-v1.0-wd01.md` is a frozen draft snapshot, regenerated
  wholesale only when cutting the next WD.
  The Rust types only mirror the per-target CBOR schemas defined there. Change
  the format spec-first (`spec:` commits), then update the types to match; never
  let the types redefine or silently drift from the spec.

<!-- Shared rules are in the .agent submodule. If .agent/ is empty:
     git submodule update --init --recursive -->
@.agent/karpathy/CLAUDE.md
@.agent/pichi.md
@.agent/rust.md
