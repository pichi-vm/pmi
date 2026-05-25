# `cca` Target

The `cca` target is the Arm CCA (Confidential Compute Architecture)
launch path. It is built on [`vm`](vm.md): inherits vm's base
launch model and admits the [`load`](core.md#load) and
[`fill`](core.md#fill) actions with CCA-specific kinds. It uses
a `vcpu` top-level field that the VMM applies as the BSP REC
parameters via `RMI_REC_CREATE` at step 2.

## PE section

A VMM targeting `cca` reads the `.pmi.cca` PE section. The section
MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is
absent, the image does not support `cca` and the VMM MUST refuse to
launch.

## Schema

```cddl
cca = {
  "version" => uint,                     ; schema version (1)
  "vcpu"    => vcpu-aarch64,             ; BSP REC params; CCA is aarch64 only
  "actions" => [+ cca-action],           ; ordered launch recipe
}

cca-action = load / fill
```

The [core validation rules](core.md#validation) apply. `version` MUST be `1`. The
`cca` target adds no further validation rules.

## Launch model

The `cca` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with the following Arm CCA behavior layered on:

| Step          | API                                    | Inputs                                                              |
| ------------- | -------------------------------------- | ------------------------------------------------------------------- |
| 2. Initialize | `RMI_REALM_CREATE` then `RMI_REC_CREATE` (BSP) | host-supplied realm parameters; spec's `vcpu` field for the BSP REC |
| 3. Update     | `RMI_DATA_CREATE` / `RMI_DATA_CREATE_UNKNOWN` per action | each action in array order; selection by action kind                |
| 4. Finalize   | `RMI_REALM_ACTIVATE`                   | locks RIM                                                           |

RIM extension is reproducible from the image bytes per the granule-submission
ordering fixed by the core [`load`](core.md#load) and [`fill`](core.md#fill)
procedures.

## Realm parameters

Realm parameters (feature flags, hash algorithm, REC count, Realm
Personalization Value) are **host-supplied** — the VMM accepts them
via VMM-defined input (CLI flag, config file, etc.) and passes them
to `RMI_REALM_CREATE`. PMI does not carry them. Upper layers that
need to bind specific realm parameters to the image can declare the
expected bytes in measured PE sections via the
[Extensions namespace](extensions.md#namespacing) and require the VMM
to submit them verbatim.

CCA does not currently define a signed launch identity equivalent
to SEV's `id-block` / `id-auth`. The PMI image carries no identity
material; verifiers bind to RIM plus the Realm Token.

## `vcpu` field

The `vcpu` field carries the BSP REC parameters the VMM applies at
step 2 via `RMI_REC_CREATE`. The schema is vm's
[`vcpu-aarch64`](vm.md#vcpu-aarch64); CCA is aarch64 only.

The BSP REC is created with `runnable = RUNNABLE`. Its parameters
(notably PC, GPRs, and the system registers exposed by
`vcpu-aarch64`) are measured into RIM. Secondary RECs are created
non-runnable by the VMM (independent of PMI) and brought up at
runtime by the realm via `PSCI_CPU_ON`.

## Actions

The `cca` target admits the [`load`](core.md#load) and
[`fill`](core.md#fill) actions.

### `load`

`cca` defines one `load` kind:

- **`default`**: the VMM submits the PE section's granules via
  `RMI_DATA_CREATE`. The granule content is copied from a
  non-secure source granule to the destination granule, hashed,
  and the hash is extended into RIM.

### `fill`

`cca` defines no `fill` kinds. Upper layers MAY register their
own through `fill`'s extension point; see
[Extensions](extensions.md#4-action-defined-extension-points).

## Status

The CCA target binding is a working draft. Open items:

- Whether `vcpu-aarch64`'s register set fully captures the BSP REC
  parameters CCA measures into RIM, or whether CCA-specific fields
  need their own schema (e.g., the `runnable` flag at REC creation,
  which RMM 1.0-rel0 includes in RIM only when set).
- Auxiliary REC granules (count returned by `RMI_REC_AUX_COUNT`):
  per-platform and per-realm, allocated by the VMM. Runtime
  allocator output, by design out of PMI's bindings.
- REM (Realm Extensible Measurement) initial state: REMs are
  runtime-extended by the realm; whether the spec needs image-side
  declaration of expected REM extensions is open.
