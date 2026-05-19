# `cca` Target

The `cca` target is the Arm CCA (Confidential Compute Architecture)
launch path. It is built on [`vm`](vm.md): inherits vm's base launch
model, extends vm's [`load`](vm.md#load-action) and
[`fill`](vm.md#fill-action) actions with CCA-specific kinds, and uses
a `vcpu` top-level field that the VMM applies as the BSP REC
parameters via `RMI_REC_CREATE` at step 3.

## PE section

A VMM targeting `cca` reads the `.pmi.cca` PE section. The section
MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is
absent, the image does not support `cca` and the VMM MUST refuse to
launch.

## Schema

```cddl
cca = {
  "version" => uint,                     ; schema version (1)
  "dtb"     => tstr,                     ; PE section name; see dtb.md
  "vcpu"    => vcpu-aarch64,             ; BSP REC params; CCA is aarch64 only
  "actions" => [+ cca-action],           ; ordered launch recipe (step 4)
}

cca-action = load / fill
```

VMMs MUST reject sections with an unrecognized `version`, an unknown
top-level key, an unknown action `type` value, or an unknown action
`kind` value.

## Launch model

The `cca` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with the following Arm CCA behavior layered on:

| Step          | API                                    | Inputs                                                              |
| ------------- | -------------------------------------- | ------------------------------------------------------------------- |
| 3. Initialize | `RMI_REALM_CREATE` then `RMI_REC_CREATE` (BSP) | host-supplied realm parameters; spec's `vcpu` field for the BSP REC |
| 4. Update     | `RMI_DATA_CREATE` / `RMI_DATA_CREATE_UNKNOWN` per action | each action in array order; selection by action kind                |
| 5. Finalize   | `RMI_REALM_ACTIVATE`                   | locks RIM                                                           |

Within each step-4 action's PE section the VMM submits granules from
the lowest GPA to the highest, so RIM extension is deterministic for
a given action ordering.

## Realm parameters

Realm parameters are **host-supplied** — the VMM accepts them via
VMM-defined input (CLI flag, config file, etc.), which is out of
scope for PMI. These include:

- Feature flags (LPA2, SVE vector length, number of breakpoints and
  watchpoints, etc.), constrained by what `RMI_FEATURES` reports the
  platform supports.
- Hash algorithm used for RIM/REM extension.
- Number of RECs.
- Realm Personalization Value (RPV) — 64 bytes supplied by the
  deployer.

The VMM passes these to `RMI_REALM_CREATE`. The features specified
at realm creation are measured into RIM; the RPV is not measured but
appears in the Realm Token for verifier inspection.

CCA does not currently define a signed launch identity equivalent to
SEV's `id-block` / `id-auth`. The PMI image carries no identity
material; verifiers bind to RIM plus the Realm Token (which
incorporates the RPV).

## `vcpu` field

The `vcpu` field carries the BSP REC parameters the VMM applies at
step 3 via `RMI_REC_CREATE`. The schema is vm's
[`vcpu-aarch64`](vm.md#aarch64-pe-fileheader-machine--0xaa64); CCA is
aarch64 only.

The BSP REC is created with `runnable = RUNNABLE`. Its parameters
(notably PC, GPRs, and the system registers exposed by
`vcpu-aarch64`) are measured into RIM. Secondary RECs are created
non-runnable by the VMM (independent of PMI) and brought up at
runtime by the realm via `PSCI_CPU_ON`.

## `load` action

`cca` extends the [base `load` action](vm.md#load-action) with
CCA-specific kinds; the default kind is `measured`.

### Schema

```cddl
load = {
  "type"    => "load",
  "section" => tstr,                ; PE section name to load
  ? "kind"  => "measured" / "unmeasured",  ; default "measured"
}
```

### kind `measured`

The default kind. The VMM submits the PE section's granules via
`RMI_DATA_CREATE`. The granule content is copied from a non-secure
source granule to the destination granule, hashed, and the hash is
extended into RIM.

### kind `unmeasured`

The VMM submits the PE section's granules via
`RMI_DATA_CREATE_UNKNOWN`. The granule is added to the realm as
uninitialized memory — content is not transferred and the granule is
not extended into RIM. The PE section SHOULD be a Zero section, since
on-disk data is not used; if a Data or Padded section is used the
on-disk bytes are ignored.

## `fill` action

`cca` uses the [base `fill` action](vm.md#fill-action) with the
`dtbo` kind from `vm` unchanged.

### Schema

```cddl
fill = {
  "type"    => "fill",
  "section" => tstr,                ; zero PE section to populate
  "kind"    => "dtbo",
}
```

### kind `dtbo`

Same as the [base `dtbo` fill kind](vm.md#kind-dtbo). The VMM
generates the overlay and writes it to the section's GPA range; the
page is not submitted via `RMI_DATA_CREATE` and does not contribute
to RIM. See [`dtbo` overlay](vm.md#dtbo-overlay) for content and
consumer-validation rules.

## Status

The CCA target binding is a working draft. Open items:

- Whether `vcpu-aarch64`'s register set fully captures the BSP REC
  parameters CCA measures into RIM, or whether CCA-specific fields
  need their own schema (e.g., the `runnable` flag at REC creation,
  which RMM 1.0-rel0 includes in RIM only when set).
- Auxiliary REC granules (count returned by `RMI_REC_AUX_COUNT`):
  per-platform and per-realm, allocated by the VMM independently of
  PMI — currently out of PMI scope.
- REM (Realm Extensible Measurement) initial state: REMs are
  runtime-extended by the realm; whether the spec needs image-side
  declaration of expected REM extensions is open.
