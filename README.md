# PMI: Portable Machine Image

PMI is a working draft. Schemas and semantics may change.

## Motivation

A single workload increasingly needs to run as more than one
artifact shape — bare metal under UEFI, direct-boot VM, OVMF +
disk image, confidential VM under SEV-SNP / TDX / CCA — and each
CC vendor ships its own launch-recipe conventions that don't
compose. PMI is a deliberately narrow substrate (PE container +
per-target launch recipes + action mechanism) that lets one image
declare one recipe per target without replacing the existing PE
toolchain. See [spec/motivation.md](spec/motivation.md).

## PMI

A PMI image is a PE binary that, for each launch target it
supports, carries a CBOR-encoded launch recipe in a non-loaded
`.pmi.<target>` PE section. A launch recipe is a sequence of
**actions** the VMM processes in array order — primarily
[`load`](spec/load.md) (place a PE section's bytes into guest
memory) and [`fill`](spec/fill.md) (populate a GPA range with
kind-specific content). The CC targets (`sev`, `cca`, `tdx`) drive
their native firmware ABIs through these actions; the non-CC
`vm` target drives plain VMM memory loading.

### PE Constraints

PMI's substrate is the PE container, with alignment and section-
naming rules that keep PMI-specific data invisible to existing PE
tools. See [spec/pe.md](spec/pe.md).

### Targets

A target's structure — the CBOR shape, the action model, the
launch-step ordering, and the validation rules every loader MUST
enforce. See [spec/targets.md](spec/targets.md).

### Actions

The verbs a launch recipe is built from:
[`load`](spec/load.md) reads a PE section's bytes into guest
memory; [`fill`](spec/fill.md) populates a reserved GPA range with
kind-specific content. Both are extensible through their `kind`
field.

### Extensions

The namespacing convention that lets upper layers (hypervisors,
in-guest stubs, image schemas) attach layer-specific data —
registered vs unregistered prefixes, the four extension points.
See [spec/extensions.md](spec/extensions.md).

## Extensions

The following prefixes are registered with PMI. Each one is itself
a registered extension; together they cover the launch targets PMI
currently defines.

| Prefix  | Spec                       | Description                                 |
| ------- | -------------------------- | ------------------------------------------- |
| `vm`    | [spec/vm.md](spec/vm.md)   | Non-CC virtual machine target               |
| `sev`   | [spec/sev.md](spec/sev.md) | AMD SEV 3.0 (SEV-SNP) confidential VMs      |
| `tdx`   | [spec/tdx.md](spec/tdx.md) | Intel TDX confidential VMs (draft)          |
| `cca`   | [spec/cca.md](spec/cca.md) | Arm CCA confidential VMs (draft)            |

To register a new extension, open an issue or pull request against
the PMI spec repository with the proposed prefix and a link to its
spec.

## Examples

Concrete CBOR walkthroughs of PMI images across the targets above:
see [spec/examples.md](spec/examples.md).
