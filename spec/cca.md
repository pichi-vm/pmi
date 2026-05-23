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
  "vcpu"    => vcpu-aarch64,             ; BSP REC params; CCA is aarch64 only
  "actions" => [+ cca-action],           ; ordered launch recipe
}

cca-action = load / fill
```

The schema-strictness and action-array validation rules from
[`vm`](vm.md#schema) apply: unrecognized `version`, unknown key in
any defined CBOR map, unknown action `type`, unknown action `kind`,
non-existent section reference, duplicate section reference, and
overlapping `[VirtualAddress, VirtualAddress + VirtualSize)` ranges
all cause the VMM to refuse to launch.

## Parameters

The `cca` target's parameters mapped against PMI's
[categories](categories.md):

| Parameter                                          | Category           | Source         | Notes                                                                                                                                |
| -------------------------------------------------- | ------------------ | -------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `vcpu` field (BSP REC parameters)                  | Platform identity  | PMI image      | Applied at step 3 via `RMI_REC_CREATE`; measured into RIM                                                                            |
| `load` action (kind `measured`)                    | Image identity     | PMI image      | Granule content submitted via `RMI_DATA_CREATE`; hashed and extended into RIM                                                        |

### RmiRealmParams bit-by-bit

The realm parameters passed to `RMI_REALM_CREATE` mix platform
identity (liveness requirements measured into RIM), tenant identity
(the deployer-supplied RPV), and instance accidents (per-deployment
sizing). Today they are host-supplied via VMM-defined input; the
platform-identity fields need to be
[promoted to image identity](categories.md#promoting-to-image-identity)
so the image can declare them and a VMM that substitutes a different
value diverges RIM. The concrete measured fill kinds are open spec
work — see [Status](#status).

| Parameter                                          | Category           | Source     | Notes                                                                                                                                |
| -------------------------------------------------- | ------------------ | ---------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `s2sz` (IPA size)                                  | Platform identity  | Runtime    | Measured into RIM; image's address-space layout depends on it                                                                        |
| `sve_en` / `sve_vl`                                | Platform identity  | Runtime    | Liveness requirement: image's use of SVE is gated on these                                                                           |
| `num_bps` / `num_wps`                              | Platform identity  | Runtime    | Liveness requirement when image uses hardware breakpoints/watchpoints                                                                |
| `pmu_en` / `pmu_num_ctrs`                          | Platform identity  | Runtime    | Liveness requirement when image uses PMU counters                                                                                    |
| `hash_algo`                                        | Platform identity  | Runtime    | Determines RIM hash function; the verifier needs the same value to reproduce the expected RIM                                        |
| `rpv` (Realm Personalization Value, 64 bytes)      | Tenant identity    | Runtime    | Not measured into RIM; surfaced in the Realm Token for verifier inspection                                                           |
| Number of RECs                                     | Instance accidents | Runtime    | Per-deployment sizing                                                                                                                |

### RmiRecParams (non-BSP)

Secondary REC parameters beyond the BSP's `vcpu` field. Secondary
RECs are created non-runnable by the VMM and brought up at runtime
by the realm via `PSCI_CPU_ON`.

| Parameter                                          | Category           | Source     | Notes                                                                                                                                |
| -------------------------------------------------- | ------------------ | ---------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| BSP REC parameters                                 | Platform identity  | PMI image  | Carried via [`vcpu`](#vcpu-field)                                                                                                    |
| Secondary REC initial state                        | Instance accidents | Runtime    | Brought up by the realm at runtime; not in the launch contract                                                                       |
| Auxiliary REC granules (count from `RMI_REC_AUX_COUNT`) | Instance accidents | Runtime    | Per-platform / per-realm allocator output                                                                                            |

`cca` has no signed launch identity equivalent to SEV's `id` block
and no host-identity channel equivalent to SEV's `HOST_DATA`. CCA
has no separate launch-policy field — the realm-creation feature
flags are liveness requirements and classified as platform
identity above.

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
[`vcpu-aarch64`](vm.md#vcpu-aarch64); CCA is aarch64 only.

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
  ? "kind"  => "measured",  ; default "measured"
}
```

### kind `measured`

The default kind. The VMM submits the PE section's granules via
`RMI_DATA_CREATE`. The granule content is copied from a non-secure
source granule to the destination granule, hashed, and the hash is
extended into RIM.

## `fill` action

`cca` defines no fill kinds itself. The [base `fill`
action](vm.md#fill-action) is available for upper layers to use via
the [Extensions](overview.md#extensions) namespace (e.g.,
`dillo:dtbo` for a dillo-managed devicetree overlay).

## Status

The CCA target binding is a working draft. Open items:

- Whether `vcpu-aarch64`'s register set fully captures the BSP REC
  parameters CCA measures into RIM, or whether CCA-specific fields
  need their own schema (e.g., the `runnable` flag at REC creation,
  which RMM 1.0-rel0 includes in RIM only when set).
- Auxiliary REC granules (count returned by `RMI_REC_AUX_COUNT`):
  per-platform and per-realm, allocated by the VMM. These are
  [instance accidents](overview.md#categories) — runtime
  allocator output — and by design out of PMI's bindings.
- REM (Realm Extensible Measurement) initial state: REMs are
  runtime-extended by the realm; whether the spec needs image-side
  declaration of expected REM extensions is open.
