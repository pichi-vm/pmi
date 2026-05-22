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
| `dtb` field (base DTB bytes)                       | Platform identity  | PMI image  | Names the [base DTB](dtb.md); host MUST be able to satisfy every declared resource                                  |
| `load` action (kind `measured`)                    | Image identity     | PMI image  | Page bytes contribute to MRTD via `TDH.MR.EXTEND`; the PMI consumer (reset-vector occupant) is itself a measured load |
| `load` action (kind `unmeasured`)                  | Image identity     | PMI image  | Bytes are image-declared; the GPA always enters MRTD even though content does not (`TDH.MEM.PAGE.ADD` only)         |
| `fill` action (kind `dtbo`)                        | Instance accidents | Runtime    | Host-generated DT overlay; not submitted via `KVM_TDX_INIT_MEM_REGION` and does not contribute to MRTD              |
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

| Bit  | Name             | Category          | Notes                                                                                                |
| ---- | ---------------- | ----------------- | ---------------------------------------------------------------------------------------------------- |
| 0    | DEBUG            | Leftover          | Debug-enabled; deployer operational choice                                                           |
| 27   | LASS             | Platform identity | Linear Address Space Separation; image's use of LASS is gated on this bit                            |
| 28   | SEPT_VE_DISABLE | Platform identity | Disables #VE on pending Secure-EPT violations; the image's lazy-acceptance and #VE handling depend on this bit's value |
| 29   | MIGRATABLE       | Leftover          | Migratable-TD flag; deployer operational choice                                                      |
| 30   | PKS              | Platform identity | Protection Keys for Supervisor; liveness requirement when the image uses PKS                         |
| 31   | KL               | Platform identity | Key Locker; liveness requirement when the image uses Key Locker instructions                         |
| 62   | TPA              | Platform identity | TD Partitioning Architecture; liveness requirement when the image relies on partitioning             |
| 63   | PERFMON          | Platform identity | Performance Monitoring; liveness requirement when the image uses PMU                                 |
| Others | RESERVED       | N/A               | Architecturally MBZ or vendor-reserved                                                               |

Because `ATTRIBUTES` is measured into MRTD, the value the host
supplies is bound into the launch measurement. For
[attestation equivalence](overview.md#attestation-equivalence) two
conformant VMMs running the same PMI image must produce the same
MRTD, which means the same `ATTRIBUTES` value — including its
leftover bits. This is a documented gap: PMI does not yet carry a
mechanism for the image to declare its expected `ATTRIBUTES` so
that conformant VMMs can be forced to agree.

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
enumerated in [TD_PARAMS](#td_params) above. Several of those fields
are platform identity (liveness requirements measured into MRTD) and
need a PMI carriage mechanism so the image can declare them; that is
open work — see [Status](#status).

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
