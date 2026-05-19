# `tdx` Target

The `tdx` target is the Intel TDX launch path. It is built on
[`vm`](vm.md): inherits vm's base launch model, extends vm's
[`load`](vm.md#load-action) and [`fill`](vm.md#fill-action) actions
with TDX-specific kinds, and uses a `vcpu` top-level field that the
VMM applies via `KVM_TDX_INIT_VCPU` at step 3.

## PE section

A VMM targeting `tdx` reads the `.pmi.tdx` PE section. The section
MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is
absent, the image does not support `tdx` and the VMM MUST refuse to
launch.

## Schema

```cddl
tdx = {
  "version" => uint,                     ; schema version (1)
  "dtb"     => tstr,                     ; PE section name; see dtb.md
  "vcpu"    => vcpu-x64,                 ; BSP register state; TDX is x86-64 only
  "actions" => [+ tdx-action],           ; ordered launch recipe (step 4)
}

tdx-action = load / fill
```

VMMs MUST reject sections with an unrecognized `version`, an unknown
top-level key, an unknown action `type` value, or an unknown action
`kind` value.

## Launch model

The `tdx` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with the following Intel TDX behavior layered on:

| Step          | API                                       | Inputs                                                                |
| ------------- | ----------------------------------------- | --------------------------------------------------------------------- |
| 3. Initialize | `KVM_TDX_INIT_VM` then `KVM_TDX_INIT_VCPU` | host-supplied TD parameters; spec's `vcpu` field for BSP registers     |
| 4. Update     | `KVM_TDX_INIT_MEM_REGION` per action       | each action in array order; `KVM_TDX_MEASURE_MEMORY_REGION` flag set per the action's kind |
| 5. Finalize   | `KVM_TDX_FINALIZE_VM`                      | locks MRTD                                                            |

Within each step-4 action's PE section the VMM submits pages from the
lowest GPA to the highest, so MRTD extension is deterministic for a
given action ordering.

## TD parameters

TD parameters are **host-supplied** — the VMM accepts them via
VMM-defined input (CLI flag, config file, etc.), which is out of
scope for PMI. These include:

- `ATTRIBUTES` — TD attributes flags (DEBUG, SEPT_VE_DISABLE, etc.).
- `XFAM` — extended feature mask (XCR0 / XSS bits the TD is allowed
  to use).
- CPUID configuration — host-determined CPUID values the TDX module
  validates against the actual processor.
- `MRCONFIGID`, `MROWNER`, `MROWNERCONFIG` — 48-byte deployer fields,
  not measured into MRTD but surfaced in the attestation report.

The VMM passes these to `KVM_TDX_INIT_VM`.

TDX does not currently define a signed launch identity equivalent to
SEV's `id-block` / `id-auth`. The PMI image carries no identity
material; verifiers bind to MRTD plus the deployer fields in the
attestation report.

## `vcpu` field

The `vcpu` field carries the BSP register state the VMM applies at
step 3 via `KVM_TDX_INIT_VCPU`. The schema is vm's
[`vcpu-x64`](vm.md#x86-64-pe-fileheader-machine--0x8664); TDX is
x86-64 only.

The VMM applies the register map at step 3, before processing the
actions array. The TDX module constrains which initial register
values are honored — `KVM_TDX_INIT_VCPU` exposes fewer knobs than
SEV-SNP's VMSA — so not every `vcpu-x64` register is honored. The
mapping from `vcpu-x64` to TDX init parameters is implementation-
defined within this constraint.

## `load` action

`tdx` extends the [base `load` action](vm.md#load-action) with
TDX-specific kinds; the default kind is `measured`.

### Schema

```cddl
load = {
  "type"    => "load",
  "section" => tstr,                ; PE section name to load
  ? "kind"  => "measured" / "unmeasured",  ; default "measured"
}
```

### kind `measured`

The default kind. The VMM submits the PE section's pages via
`KVM_TDX_INIT_MEM_REGION` with the `KVM_TDX_MEASURE_MEMORY_REGION`
flag set — `TDH.MEM.PAGE.ADD` followed by `TDH.MR.EXTEND` per
256-byte chunk. Both the GPA and the page content contribute to MRTD.

### kind `unmeasured`

The VMM submits the PE section's pages via `KVM_TDX_INIT_MEM_REGION`
without the `KVM_TDX_MEASURE_MEMORY_REGION` flag — `TDH.MEM.PAGE.ADD`
only. The GPA is inserted into MRTD but the page content is not
extended.

Note: TDX always inserts the GPA of every initial page into MRTD, so
this kind is "content-unmeasured" rather than "fully unmeasured";
there is no TDX equivalent of SEV-SNP's `PAGE_TYPE_UNMEASURED`, which
omits the page from the digest entirely.

## `fill` action

`tdx` uses the [base `fill` action](vm.md#fill-action) with the
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
page is not submitted via `KVM_TDX_INIT_MEM_REGION` and does not
contribute to MRTD. See [`dtbo` overlay](vm.md#dtbo-overlay) for
content and consumer-validation rules.

## Status

The TDX target binding is a working draft. Open items:

- Whether TDVF / td-shim is loaded via PMI `load` actions or supplied
  out-of-band by the VMM (likely the former, with the firmware as one
  more `measured` load).
- The precise `vcpu-x64` → `KVM_TDX_INIT_VCPU` mapping, given that
  TDX vCPU init exposes fewer knobs than the full architectural
  register set.
- TD HOBs (the platform configuration TDVF reads at boot): currently
  out of PMI scope, constructed by TDVF from the merged DTB + dtbo
  via guest-side code. Whether a TDX-specific `fill` kind for TD HOBs
  would be useful is an open question.
- Whether RTMR runtime extensions need image-side declaration; the
  working assumption is no — RTMRs are extended at runtime by the
  guest, not at launch.
