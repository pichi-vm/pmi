# `tdx` Target

The `tdx` target is the Intel TDX launch path. It is built on
[`vm`](vm.md): inherits vm's base launch model and the
[`load`](vm.md#load-action) / [`fill`](vm.md#fill-action) actions with
TDX-specific kinds.

The image carries an in-TD PMI consumer that takes the role TDVF
plays in non-PMI TDX guests: it occupies the architectural reset
vector, performs vCPU rendezvous, reads any upper-layer metadata
the image declares, and hands off to the kernel. The PMI consumer's
implementation is out of scope for this spec.

## PE section

A VMM targeting `tdx` reads the `.pmi.tdx` PE section. The section
MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is
absent, the image does not support `tdx` and the VMM MUST refuse to
launch.

## Schema

```cddl
tdx = {
  "version" => uint,                     ; schema version (1)
  "actions" => [+ tdx-action],           ; ordered launch recipe
}

tdx-action = load / fill
```

The schema-strictness and action-array validation rules from
[`vm`](vm.md#schema) apply: unrecognized `version`, unknown key in
any defined CBOR map, unknown action `type`, unknown action `kind`,
non-existent section reference, duplicate section reference, and
overlapping `[VirtualAddress, VirtualAddress + VirtualSize)` ranges
all cause the VMM to refuse to launch.

There is no `vcpu` field: TDX vCPU initial register state is set by
the TDX module per the TDX architecture (see [Boot vCPU
initialization](#boot-vcpu-initialization) below).

## Parameters

The `tdx` target's parameters mapped against PMI's
[categories](categories.md):

| Parameter                                          | Category           | Source     | Notes                                                                                                                |
| -------------------------------------------------- | ------------------ | ---------- | -------------------------------------------------------------------------------------------------------------------- |
| `load` action (kind `measured`)                    | Image identity     | PMI image  | Page bytes contribute to MRTD via `TDH.MR.EXTEND`; the PMI consumer (reset-vector occupant) is itself a measured load |
| EPTP controls                                      | Instance accidents | Runtime    | VMM-internal; not visible to the guest as hardware shape                                                             |

### TD_PARAMS

The `TD_PARAMS` structure passed to `KVM_TDX_INIT_VM` is currently
host-supplied and not represented in PMI. Several of its fields name
liveness requirements the image depends on, so they classify as
platform identity and need a PMI carriage mechanism (open work).
Other fields are deployer-bound (tenant identity) or
operationally-bound (leftover).

| Field                                              | Category           | Source     | Notes                                                                                                                |
| -------------------------------------------------- | ------------------ | ---------- | -------------------------------------------------------------------------------------------------------------------- |
| `ATTRIBUTES` (see [bit-by-bit](#attributes-bit-by-bit)) | (mixed)            | Runtime    | Measured into MRTD; bits split between platform identity and leftover                                                |
| `XFAM` (extended-feature mask)                     | Platform identity  | Runtime    | Authorizes XCR0/XSS bits the TD may set; enabled bits are liveness requirements (the image uses SVE/AVX/etc.)        |
| CPUID configuration                                | (mixed)            | Runtime    | TDX-module validated against the actual processor; image-relevant bits are platform identity, host-curated bits are leftover (analogous to [SEV `cpuid`](sev.md#kind-cpuid)) |
| `MRCONFIGID` (48 bytes)                            | Tenant identity    | Runtime    | Not measured into MRTD; surfaced in TDREPORT                                                                          |
| `MROWNER` (48 bytes)                               | Tenant identity    | Runtime    | Not measured into MRTD; surfaced in TDREPORT                                                                          |
| `MROWNERCONFIG` (48 bytes)                         | Tenant identity    | Runtime    | Not measured into MRTD; surfaced in TDREPORT                                                                          |

There is no signed launch identity equivalent to SEV's `id` block and
no host-identity channel equivalent to SEV's `HOST_DATA`.

### ATTRIBUTES bit-by-bit

The 64-bit `ATTRIBUTES` field passed to `KVM_TDX_INIT_VM` is
measured into MRTD and mixes liveness requirements (platform
identity — the image won't run correctly without the bit's named
value) with deployer operational choices (leftover).

| Bit  | Name             | Category                | Notes                                                                                                |
| ---- | ---------------- | ----------------------- | ---------------------------------------------------------------------------------------------------- |
| 0    | DEBUG            | Measured leftover (topology A) | Operationally a deployer choice, but bound into MRTD by TDX; needs [promotion](categories.md#promoting-to-image-identity) |
| 27   | LASS             | Platform identity       | Linear Address Space Separation; image's use of LASS is gated on this bit                            |
| 28   | SEPT_VE_DISABLE | Platform identity       | Disables #VE on pending Secure-EPT violations; the image's lazy-acceptance and #VE handling depend on this bit's value |
| 29   | MIGRATABLE       | Measured leftover (topology A) | Operationally a deployer choice, but bound into MRTD by TDX; needs [promotion](categories.md#promoting-to-image-identity) |
| 30   | PKS              | Platform identity       | Protection Keys for Supervisor; liveness requirement when the image uses PKS                         |
| 31   | KL               | Platform identity       | Key Locker; liveness requirement when the image uses Key Locker instructions                         |
| 62   | TPA              | Platform identity       | TD Partitioning Architecture; liveness requirement when the image relies on partitioning             |
| 63   | PERFMON          | Platform identity       | Performance Monitoring; liveness requirement when the image uses PMU                                 |
| Others | RESERVED       | N/A                     | Architecturally MBZ or vendor-reserved                                                               |

Because `ATTRIBUTES` is measured into MRTD, the value the host
supplies is bound into the launch measurement. For
[attestation equivalence](overview.md#attestation-equivalence) two
conformant VMMs running the same PMI image must produce the same
MRTD, which means the same `ATTRIBUTES` value — including its
leftover bits. PMI's path to close this gap is to
[promote `ATTRIBUTES` to image identity](categories.md#promoting-to-image-identity)
via a measured fill: the image declares the expected value in a PE
section, the VMM submits exactly those bytes to `KVM_TDX_INIT_VM`,
and a VMM that substitutes a different value diverges MRTD. The
concrete measured fill kind is open spec work.

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
VMM-defined input (CLI flag, config file, etc.) and passes them to
`KVM_TDX_INIT_VM`. The fields and their PMI category mapping are
enumerated in [TD_PARAMS](#td_params) above. Fields that are
platform identity (liveness requirements measured into MRTD) need
to be [promoted to image identity](categories.md#promoting-to-image-identity)
so the image can declare them; the concrete measured fill kinds are
open spec work — see [Status](#status).

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
and hands off to the kernel. The PMI consumer is loaded as a
`measured` load action and is therefore part of the launch identity
(MRTD). Upper layers that need additional reset-vector
responsibilities (DTB inspection, dtbo merge, consumer validation
against host-supplied data) layer them onto the PMI consumer via
the [Extensions](overview.md#extensions) namespace.

This spec describes the consumer's contract but does not mandate an
implementation. Image authors may use any consumer that satisfies the
contract; PMI consumers for TDX are expected to be lightweight (much
smaller than TDVF).

## `load` action

`tdx` extends the [base `load` action](vm.md#load-action) with
TDX-specific kinds; the default kind is `measured`.

### Schema

```cddl
load = {
  "type"    => "load",
  "section" => tstr,                ; PE section name to load
  ? "kind"  => "measured",  ; default "measured"
}
```

### kind `measured`

The default kind. The VMM submits the PE section's pages via
`KVM_TDX_INIT_MEM_REGION` with the `KVM_TDX_MEASURE_MEMORY_REGION`
flag set — `TDH.MEM.PAGE.ADD` followed by `TDH.MR.EXTEND` per
256-byte chunk. Both the GPA and the page content contribute to MRTD.

## `fill` action

`tdx` defines no fill kinds itself. The [base `fill`
action](vm.md#fill-action) is available for upper layers to use via
the [Extensions](overview.md#extensions) namespace (e.g.,
`dillo:dtbo` for a dillo-managed devicetree overlay).

Note: PMI deliberately does not define a `td-hob` fill kind. The TD
HOB mechanism is TDVF-specific and would allow the host to supply
unconstrained platform info to the guest. Upper layers that need
platform-definition delivery use their own namespaced fill kinds
with their own consumer-validation rules.

## Status

The TDX target binding is a working draft. Open items:

- A reference PMI consumer for TDX (out of spec scope, but needed for
  the binding to be usable in practice). Expected responsibilities:
  reset-vector occupation, vCPU rendezvous, lazy memory acceptance,
  MMIO handling via `TDG.VP.VMCALL<#VE.RequestMMIO>`, CPUID page
  consumption, and kernel handoff. Upper-layer responsibilities
  (DTB inspection, dtbo merge, consumer validation) are layered on
  top of the base PMI consumer.
- The exact CDDL constraint on PE section `VirtualAddress` for the
  reset-vector-occupying load — whether the spec should mandate the
  architectural reset vector address or leave it to the consumer's
  metadata.
- Whether RTMR runtime extensions need image-side declaration; the
  working assumption is no — RTMRs are extended at runtime by the
  guest (the PMI consumer or the kernel), not at launch.
