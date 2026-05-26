# `cca` Extension

**Prefix:** `cca`.

The `cca` extension provides the essential functionality for launching a PMI as
a confidential virtual machine on Arm CCA (Confidential Compute Architecture).
It defines two extension points:

1. The new target [`.pmi.cca`](#1-new-target-pmicca).
2. The new target attribute [`cca:vcpu`](#2-new-target-attribute-ccavcpu).

The `cca` target is built on [`vm`](vm.md): it inherits vm's base launch model
and admits the [`load`](core.md#load) and [`fill`](core.md#fill) actions with
CCA-specific kinds.

## 1. New target: `.pmi.cca`

The `.pmi.cca` PE section carries the `cca` target spec, subject to the
[page granularity](granularity.md) rules.

### Launch model

The `cca` target follows the [core launch model](core.md#launch-model), layering
the Arm CCA firmware ABI onto the five ordered steps:

1. Read the `.pmi.cca` PE section.
2. `RMI_REALM_CREATE` then `RMI_REC_CREATE` for the BSP, with the host-supplied
   realm parameters (see [Realm parameters](#realm-parameters)) and the spec's
   `cca:vcpu` field (see [§2](#2-new-target-attribute-ccavcpu)).
3. Process each entry in `actions` in array order via `RMI_DATA_CREATE` /
   `RMI_DATA_CREATE_UNKNOWN`, selected by the action's kind.
4. `RMI_REALM_ACTIVATE`, which locks RIM.
5. Start the guest.

RIM extension is reproducible from the image bytes per the granule-submission
ordering fixed by the core [`load`](core.md#load) and [`fill`](core.md#fill)
procedures.

### Keys

The `.pmi.cca` CBOR map follows the [core target shape](core.md#shape). Its
`version` MUST be `1`. It adds one required key:

- **`cca:vcpu`** — BSP REC parameters (see
  [§2](#2-new-target-attribute-ccavcpu)).

### Validation

The [core validation rules](core.md#validation) apply. The `cca` target adds no
further validation rules.

### Realm parameters

Realm parameters (feature flags, hash algorithm, REC count, Realm
Personalization Value) are **host-supplied** — the VMM accepts them via
VMM-defined input (CLI flag, config file, etc.) and passes them to
`RMI_REALM_CREATE`. PMI does not carry them.

CCA does not currently define a signed launch identity equivalent to SEV's
`sev:id`. The PMI image carries no identity material; verifiers bind to RIM plus
the Realm Token.

### `load`

On `cca`, the `default` kind submits the section's granules via
`RMI_DATA_CREATE`. The granule content is copied from a non-secure source
granule to the destination granule, hashed, and the hash is extended into RIM.

## 2. New target attribute: `cca:vcpu`

The `cca:vcpu` field carries the BSP REC parameters the VMM applies at launch
step 2 via `RMI_REC_CREATE`. The schema is vm's
[`vcpu-aarch64`](vm.md#vcpu-aarch64); CCA is aarch64 only.

The BSP REC is created with `runnable = RUNNABLE`. Its parameters (notably PC,
GPRs, and the system registers exposed by `vcpu-aarch64`) are measured into RIM.
Secondary RECs are created non-runnable by the VMM (independent of PMI) and
brought up at runtime by the realm via `PSCI_CPU_ON`.

## Example

A `.pmi.cca` that loads a kernel payload, supplies a host devicetree, and sets
the BSP REC parameters:

```cbor-diag
{
  "version": 1,
  "cca:vcpu": {"pc": 0x100000, "x0": 0x80000},
  "actions": [
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"},
    {"type": "fill", "section": ".dtb", "kind": "dtb"}
  ]
}
```

After `RMI_REALM_CREATE` and `RMI_REC_CREATE` (applying `cca:vcpu` to the BSP
REC), each `default` load submits granules via `RMI_DATA_CREATE`, extending RIM
with `.linux`, `.initrd`, and `.cmdline`. The `.dtb` is placed as an unmeasured
granule. `RMI_REALM_ACTIVATE` locks RIM, and the realm starts at the BSP REC's
`pc`, where it validates and consumes the devicetree before booting the kernel.
