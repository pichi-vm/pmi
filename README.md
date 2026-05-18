# PMI: Portable Machine Image

PMI (Portable Machine Image) is a format for low-level virtual machine
images. One PE binary describes how to launch on bare metal, in a non-CC VM,
and in a confidential VM across multiple CC targets. The image declares the
platform layout it requires; the VMM conforms or refuses to launch.

PMI is a working draft. Schemas and semantics may change.

## Read the spec

- [Motivation](spec/motivation.md) — Problem and goals
- [Overview](spec/overview.md) — Architecture and reading guide
- [Examples](spec/examples.md) — Concrete CBOR walkthroughs

### Targets

- [`vm`](spec/vm.md) — non-CC VMs
- [`sev`](spec/sev.md) — AMD SEV-SNP
- [`tdx`](spec/tdx.md), [`cca`](spec/cca.md) — TODO

### Reference

- [PE constraints](spec/pe.md)
- [Base DTB](spec/dtb.md)
- [`load` action](spec/load.md)
- [`dtbo` action](spec/dtbo.md)
