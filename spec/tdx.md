# `tdx` Target

The `tdx` target is the Intel TDX launch path. It is built on
[`vm`](vm.md): inherits vm's base launch model and admits the
[`load`](core.md#load) and [`fill`](core.md#fill) actions
with TDX-specific kinds.

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

The [core validation rules](core.md#validation) apply. `version` MUST be `1`. The
`tdx` target adds no further validation rules.

There is no `vcpu` field: TDX vCPU initial register state is set by
the TDX module per the TDX architecture (see [Boot vCPU
initialization](#boot-vcpu-initialization) below).

## Launch model

The `tdx` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with the following Intel TDX behavior layered on:

| Step          | API                                       | Inputs                                                                |
| ------------- | ----------------------------------------- | --------------------------------------------------------------------- |
| 2. Initialize | `KVM_TDX_INIT_VM` then `KVM_TDX_INIT_VCPU` | host-supplied TD parameters                                            |
| 3. Update     | `KVM_TDX_INIT_MEM_REGION` per action       | each action in array order; `KVM_TDX_MEASURE_MEMORY_REGION` flag set per the action's kind |
| 4. Finalize   | `KVM_TDX_FINALIZE_VM`                      | locks MRTD                                                            |

MRTD extension is reproducible from the image bytes per the page-submission
ordering fixed by the core [`load`](core.md#load) and [`fill`](core.md#fill)
procedures.

## TD parameters

`TD_PARAMS` (including `ATTRIBUTES`, `XFAM`, CPUID configuration,
and the `MRCONFIGID` / `MROWNER` / `MROWNERCONFIG` deployer fields)
is **host-supplied** — the VMM accepts it via VMM-defined input
(CLI flag, config file, etc.) and passes it to `KVM_TDX_INIT_VM`.
PMI does not carry it. Upper layers that need to bind specific
`TD_PARAMS` fields to the image can declare the expected bytes in
measured PE sections via the [Extensions namespace](extensions.md#namespacing)
and require the VMM to submit them verbatim. Because
`TD_PARAMS` is measured into MRTD, that binding is enforced
cryptographically — a VMM that substitutes a different value
diverges MRTD.

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
`default` load action and is therefore part of the launch identity
(MRTD). Upper layers that need additional reset-vector
responsibilities — platform-metadata inspection, host-data merge,
consumer validation against host-supplied bytes — layer them onto
the PMI consumer via the [Extensions namespace](extensions.md#namespacing).

This spec describes the consumer's contract but does not mandate an
implementation. Image authors may use any consumer that satisfies the
contract; PMI consumers for TDX are expected to be lightweight (much
smaller than TDVF).

## Actions

The `tdx` target admits the [`load`](core.md#load) and
[`fill`](core.md#fill) actions.

### `load`

`tdx` defines one `load` kind:

- **`default`**: the VMM submits the PE section's pages via
  `KVM_TDX_INIT_MEM_REGION` with the
  `KVM_TDX_MEASURE_MEMORY_REGION` flag set — `TDH.MEM.PAGE.ADD`
  followed by `TDH.MR.EXTEND` per 256-byte chunk. Both the GPA
  and the page content contribute to MRTD.

### `fill`

`tdx` defines no `fill` kinds. Upper layers MAY register their
own through `fill`'s extension point; see
[Extensions](extensions.md#4-action-defined-extension-points).

Note: PMI deliberately does not define a `td-hob` fill kind. The
TD HOB mechanism is TDVF-specific and would allow the host to
supply unconstrained platform info to the guest. Upper layers
that need platform-definition delivery use their own namespaced
fill kinds with their own consumer-validation rules.

## Status

The TDX target binding is a working draft. Open items:

- A reference PMI consumer for TDX (out of spec scope, but needed for
  the binding to be usable in practice). Expected responsibilities:
  reset-vector occupation, vCPU rendezvous, lazy memory acceptance,
  MMIO handling via `TDG.VP.VMCALL<#VE.RequestMMIO>`, CPUID page
  consumption, and kernel handoff. Upper-layer responsibilities
  (platform-metadata inspection, host-data merge, consumer
  validation) are layered on top of the base PMI consumer.
- The exact CDDL constraint on PE section `VirtualAddress` for the
  reset-vector-occupying load — whether the spec should mandate the
  architectural reset vector address or leave it to the consumer's
  metadata.
- Whether RTMR runtime extensions need image-side declaration; the
  working assumption is no — RTMRs are extended at runtime by the
  guest (the PMI consumer or the kernel), not at launch.
