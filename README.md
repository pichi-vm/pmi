# PMI: Portable Machine Image

PMI is a working draft. Schemas and semantics may change.

## Motivation

A single workload increasingly needs to run as more than one shape
of artifact. The same kernel + initrd + command line is expected
to launch on bare metal under UEFI, as a direct-boot VM where the
VMM extracts the kernel, under guest firmware (OVMF) that loads
the kernel from a virtual disk, and inside a confidential VM under
SEV-SNP, TDX, or CCA. Each of those shapes has its own image
format conventions today (PE/UKI assumes UEFI, the Linux boot
protocol assumes VMM extraction, IGVM assumes paravisor-style
confidential boot, every cloud has ad-hoc SEV/TDX conventions),
which forces image authors to publish N artifacts where they
wanted to publish one. PMI's response: a single PE binary that
declares one launch recipe per target it supports, with PE-level
selection so the same bytes work everywhere.

Each confidential-computing target ships its own conventions for
the launch recipe — Intel TDX HOBs, IGVM, QEMU/libkrun/cloud
hypervisor variants. None compose. A workload that wants to run
SEV on AWS, TDX on Azure, and CCA on tenant-owned Arm hardware
ends up maintaining three image-build paths, three verifier
integrations, three consumer stubs. PMI's response: one CBOR wire
format and one action mechanism that drives every target's native
firmware ABI, so an image author learns one shape and ships one
image to N targets.

Image formats are not consumed by one tool; producers, VMMs,
in-guest stubs, verifiers, inspectors, and signers all touch them.
Replacing PE would force every layer to be re-implemented and lose
decades of operational maturity (`objcopy`, `sbsign`, `ukify`,
`systemd-stub`, every UEFI loader). PMI's response: extend PE
rather than replace it, so the existing toolchain works on PMI
images unchanged, and the new tooling (CBOR parser, signer,
inspector) has narrow contracts that compose across targets.

The substrate scope is deliberately narrow: PE container + per-
target launch recipes + action mechanism. Platform semantics,
attestation policy, host-conformance checking, and the
measured-vs-unmeasured boundary as a security argument belong to
upper-layer specs (e.g., dillo) that build on top of PMI.

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
