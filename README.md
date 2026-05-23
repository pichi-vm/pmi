# PMI: Portable Machine Image

PMI (Portable Machine Image) is a format for low-level virtual machine
images. One PE binary describes how to launch on bare metal, in a non-CC VM,
and in a confidential VM across multiple CC targets. PMI is a narrow
substrate: a PE container, per-target CBOR launch recipes, and an action
mechanism that drives the firmware ABIs each target exposes. Platform
semantics, attestation policy, and host-conformance live in upper-layer
specs (e.g., dillo) that build on top of PMI.

PMI is a working draft. Schemas and semantics may change.

## Read the spec

- [Motivation](spec/motivation.md) — Problem and goals
- [Overview](spec/overview.md) — Architecture and reading guide
- [Extensions](spec/extensions.md) — Common target shape and upper-layer extension points

### Actions

- [`load`](spec/load.md) — load a PE section's bytes into guest memory
- [`fill`](spec/fill.md) — populate a reserved GPA range with kind-specific content

### Targets

- [`vm`](spec/vm.md) — non-CC VMs
- [`sev`](spec/sev.md) — AMD SEV-SNP
- [`tdx`](spec/tdx.md) — Intel TDX (draft)
- [`cca`](spec/cca.md) — Arm CCA (draft)

### Reference

- [PE constraints](spec/pe.md)
- [Examples](spec/examples.md) — Concrete CBOR walkthroughs
