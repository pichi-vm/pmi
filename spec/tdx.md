# `tdx` Target

The `tdx` target is the Intel TDX launch path. It is built on
[`vm`](vm.md): inherits vm's base launch model and the
[`load`](vm.md#load-action) / [`fill`](vm.md#fill-action) actions with
TDX-specific kinds.

PMI's platform-definition inversion applies on TDX. The image declares
its platform via a base DTB (measured into MRTD) and the runtime slice
via a `dtbo` fill (host-decided, consumer-validated). The image
provides an in-TD PMI consumer that takes the role TDVF plays in
non-PMI TDX guests; the consumer reads the merged DTB + dtbo, applies
PMI's [consumer validation](vm.md#consumer-validation-normative), and
hands off to the kernel. The PMI consumer's implementation is out of
scope for this spec.

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
  "actions" => [+ tdx-action],           ; ordered launch recipe (step 4)
}

tdx-action = load / fill
```

VMMs MUST reject sections with an unrecognized `version`, an unknown
top-level key, an unknown action `type` value, or an unknown action
`kind` value.

There is no `vcpu` field: TDX vCPU initial register state is set by
the TDX module per the TDX architecture (see [Boot vCPU
initialization](#boot-vcpu-initialization) below).

## Launch model

The `tdx` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with the following Intel TDX behavior layered on:

| Step          | API                                       | Inputs                                                                |
| ------------- | ----------------------------------------- | --------------------------------------------------------------------- |
| 3. Initialize | `KVM_TDX_INIT_VM` then `KVM_TDX_INIT_VCPU` | host-supplied TD parameters                                            |
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
SEV's `id` field. The PMI image carries no identity material;
verifiers bind to MRTD plus the deployer fields in the attestation
report.

## Boot vCPU initialization

TDX vCPU initial register state is set by the TDX module per the TDX
architecture. All vCPUs begin execution simultaneously at the x86
architectural reset vector (`0xFFFFFFF0`); there is no INIT/SIPI
mechanism. The hypervisor's `KVM_TDX_INIT_VCPU` ioctl conveys a small
set of values into hypervisor-controllable registers (notably `RCX`,
`RSI`, `R8`), but these are not part of PMI's contract with the guest
and the PMI consumer MUST NOT depend on them.

The image MUST carry a **PMI consumer**: a measured component that
occupies the architectural reset vector, performs vCPU rendezvous,
reads PMI's platform-definition surface from known IPAs (the merged
[base DTB](dtb.md) and [`dtbo` overlay](vm.md#dtbo-overlay)), applies
the consumer-validation rules, and hands off to the kernel.

The PMI consumer is loaded as a `measured` load action and is
therefore part of the launch identity (MRTD). The IPAs at which the
consumer expects to find the base DTB and the dtbo region are
determined at image-build time by the `VirtualAddress` fields of the
corresponding PE sections, and are baked into the (measured)
consumer. A hostile or buggy host cannot redirect the consumer; any
deviation in IPA placement breaks the consumer's expectations and the
launch fails at consumer-validation time.

This spec describes the consumer's contract but does not mandate an
implementation. Image authors may use any consumer that satisfies the
contract; PMI consumers for TDX are expected to be lightweight (much
smaller than TDVF) and to focus on PMI's consumer-validation and
DTB-to-kernel-handoff responsibilities.

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

Note: PMI deliberately does not define a `td-hob` fill kind. The TD
HOB mechanism is TDVF-specific and conflicts with PMI's
platform-definition inversion (it would allow the host to supply
unconstrained platform info to the guest). PMI consumers on TDX read
the dtbo overlay directly and do not consume HOBs.

## Status

The TDX target binding is a working draft. Open items:

- A reference PMI consumer for TDX (out of spec scope, but needed for
  the binding to be usable in practice). Expected responsibilities:
  reset-vector occupation, vCPU rendezvous, DTB + dtbo merge with
  consumer validation, lazy memory acceptance, MMIO handling via
  `TDG.VP.VMCALL<#VE.RequestMMIO>`, CPUID page consumption, and
  DT-to-ACPI translation for kernels that expect ACPI.
- The exact CDDL constraint on PE section `VirtualAddress` for the
  reset-vector-occupying load — whether the spec should mandate the
  architectural reset vector address or leave it to the consumer's
  metadata.
- Whether RTMR runtime extensions need image-side declaration; the
  working assumption is no — RTMRs are extended at runtime by the
  guest (the PMI consumer or the kernel), not at launch.
