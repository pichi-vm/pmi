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
2. `RMI_REALM_CREATE` then `RMI_REC_CREATE` for the BSP. `RmiRealmParams` is
   constructed per [Realm parameters](#realm-parameters): its measured subset
   is derived from PMI image data, and its unmeasured subset is host-supplied.
   The BSP REC is initialized from `cca:vcpu` (see
   [§2](#2-new-target-attribute-ccavcpu)).
3. Process each entry in `actions` in array order via `RMI_DATA_CREATE` /
   `RMI_DATA_CREATE_UNKNOWN`, selected by the action's kind.
4. `RMI_REALM_ACTIVATE`, which locks RIM.
5. Start the guest.

RIM extension is reproducible from the image bytes per the granule-submission
ordering fixed by the core [`load`](core.md#load) and [`fill`](core.md#fill)
procedures.

### Keys

The `.pmi.cca` CBOR map follows the [core target shape](core.md#shape). Its
`version` MUST be `1`. It adds two required keys:

- **`cca:vcpu`** — BSP REC parameters (see
  [§2](#2-new-target-attribute-ccavcpu)).
- **`cpu:profile`** — vCPU ISA baseline (see [cpu.md](cpu.md)).

### Validation

The [core validation rules](core.md#validation) apply. The `cca` target adds no
further validation rules.

### Realm parameters

`RMI_REALM_CREATE` consumes an `RmiRealmParams` structure. PMI splits it into
a measured subset (folded into RIM per DEN0137 §B4.3.9.4
`RmiRealmParamsMeasured`) and an unmeasured subset. The measured subset MUST
be a deterministic function of the PMI image so that RIM is portable across
compliant VMMs, per the [core attestation
invariant](motivation.md#2-portable-safe-platform-definition-and-attestation).
The unmeasured subset MAY be host-supplied via VMM-defined input (CLI flag,
config file, etc.); PMI does not carry it.

CCA does not currently define a signed launch identity equivalent to SEV's
`sev:id`. The PMI image carries no identity material; verifiers bind to RIM
plus the Realm Token.

#### Measured fields

The measured subset comprises `flags`, `s2sz`, `sve_vl`, `num_bps`, `num_wps`,
`pmu_num_ctrs`, `hash_algo`, and `rpv`. This draft pins the profile-derived
fields as follows:

| Field          | Value                                   | Source                                     |
| -------------- | --------------------------------------- | ------------------------------------------ |
| `flags.sve_en` | `true` iff `cpu:profile` is `armv9.x-a` | SVE/SVE2 mandatory from Armv9-A            |
| `sve_vl`       | 128 when `flags.sve_en`; otherwise 0    | Minimum legal value satisfying the profile |

The remaining measured fields — `flags.lpa2_en`, `flags.pmu_en`, `s2sz`,
`num_bps`, `num_wps`, `pmu_num_ctrs`, `hash_algo`, `rpv` — are **not yet
pinned** by this draft. See [Open measured fields](#open-measured-fields)
below.

#### Unmeasured fields

All other `RmiRealmParams` fields — including `vmid`, `rtt_base`, `rtt_level`,
`rtt_num_start`, and the REC count — are unmeasured and MAY be host-supplied
via VMM-defined input. They MAY vary per deployment without perturbing RIM.

#### Open measured fields

The following measured fields require deployment-domain expertise before they
can be pinned. Until they are, leaving them host-supplied violates the
[attestation invariant](motivation.md#2-portable-safe-platform-definition-and-attestation);
this draft is therefore not fully invariant-compliant for the `cca` target.

- `flags.lpa2_en` — whether the realm enables FEAT_LPA2 (52-bit addressing
  with 4 KiB / 16 KiB granules). FEAT_LPA2 is optional in every current
  Arm-A revision, so no profile mandates it; the choice is policy.
- `flags.pmu_en` / `pmu_num_ctrs` — whether the realm gets PMU access, and
  with how many counters. May warrant an image-author knob (e.g., a future
  `cpu:pmu` extension or `cca:pmu` attribute).
- `s2sz` — stage-2 IPA size (40 / 42 / 44 / 48 / 52 bits, subject to
  FEAT_LPA / FEAT_LPA2 availability). Bounds the realm's maximum IPA.
- `num_bps` / `num_wps` — breakpoint and watchpoint counts. Arm-A
  architectural minimum is 6 / 4; realms may need more for debuggable builds.
- `hash_algo` — SHA-256 vs SHA-512 for RIM. Determines what every downstream
  verifier must recompute.
- `rpv` — 64-byte Realm Personalization Value; image-owned identity material.

### `load`

On `cca`, the `default` kind submits the section's granules via
`RMI_DATA_CREATE`. The granule content is copied from a non-secure source
granule to the destination granule, hashed, and the hash is extended into RIM.

### `cpu:profile`

`cpu:profile` drives the SVE enable bit in `RmiRealmParams.flags` and the
`RmiRealmParams.sve_vl` field per the [Measured fields](#measured-fields)
mapping. Because these fields enter RIM, the profile is a ceiling here as well
as a floor: the VMM MUST set them deterministically from the profile so RIM
is portable across compliant VMMs. Features mandated by the profile that the
host implementation cannot satisfy cause `RMI_REALM_CREATE` to fail; the VMM
MUST refuse to launch.

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
  "cpu:profile": "armv9.2-a",
  "cca:vcpu": {"pc": 0x100000, "x0": 0x80000},
  "merged:dtb": ".dtb",
  "actions": [
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"},
    {"type": "load", "section": ".dtb"},
    {"type": "fill", "section": ".dtbo", "kind": "merged:dtbo"}
  ]
}
```

After `RMI_REALM_CREATE` and `RMI_REC_CREATE` (applying `cca:vcpu` to the BSP
REC), each `default` load submits granules via `RMI_DATA_CREATE`, extending
RIM with `.linux`, `.initrd`, `.cmdline`, and the base `.dtb`. The `.dtbo` is
placed as an unmeasured granule for the realm to validate and merge.
`RMI_REALM_ACTIVATE` locks RIM, and the realm starts at the BSP REC's `pc`,
where it validates and consumes the devicetree before booting the kernel.
