# `tdx` Extension

**Prefix:** `tdx`.

The `tdx` extension provides the essential functionality for launching a PMI as a
confidential virtual machine on Intel TDX. It defines one extension point:

1. The new target [`.pmi.tdx`](#1-new-target-pmitdx).

## 1. New target: `.pmi.tdx`

The `.pmi.tdx` PE section carries the `tdx` target spec, subject to the
[core PE constraints](constraints.md#pe-constraints).

### Launch model

The `tdx` target follows the [base launch model](vm.md#launch-model) defined by
`vm`, layering the Intel TDX firmware ABI onto the five ordered steps:

1. Read the `.pmi.tdx` PE section.
2. `KVM_TDX_INIT_VM` then `KVM_TDX_INIT_VCPU` with the host-supplied TD parameters
   (see [TD parameters](#td-parameters)).
3. Process each entry in `actions` in array order via `KVM_TDX_INIT_MEM_REGION`;
   the `KVM_TDX_MEASURE_MEMORY_REGION` flag is set per the action's kind.
4. `KVM_TDX_FINALIZE_VM`, which locks MRTD.
5. Start the guest.

MRTD extension is reproducible from the image bytes per the page-submission
ordering fixed by the core [`load`](core.md#load) and [`fill`](core.md#fill)
procedures.

### Keys

The `.pmi.tdx` CBOR map follows the [core target shape](core.md#shape). Its
`version` MUST be `1`. It adds no keys.

### Validation

The [core validation rules](core.md#validation) apply. The `tdx` target adds no
further validation rules.

### TD parameters

`TD_PARAMS` (including `ATTRIBUTES`, `XFAM`, CPUID configuration, and the
`MRCONFIGID` / `MROWNER` / `MROWNERCONFIG` deployer fields) is **host-supplied** —
the VMM passes it to `KVM_TDX_INIT_VM`; PMI does not carry it. None of it enters
MRTD, which is built only from the pages added by `load` actions, so the host
cannot perturb the image measurement through `TD_PARAMS`. Each field is attested
in its own report field; a remote verifier MUST check those separately, as it
does for SEV's launch policy.

### `load`

On `tdx`, the `default` kind submits the section's pages via
`KVM_TDX_INIT_MEM_REGION` with the `KVM_TDX_MEASURE_MEMORY_REGION` flag set —
`TDH.MEM.PAGE.ADD` followed by `TDH.MR.EXTEND` per 256-byte chunk. Both the GPA
and the page content contribute to MRTD.

TDX sets the boot vCPU at the architectural reset vector with no host-controlled
register contract, so the image MUST carry a measured **PMI consumer** loaded
there via a `default` load (and thus part of MRTD) that performs vCPU rendezvous
and hands off to the kernel. The consumer's implementation is out of scope for
this spec.

### `fill`

`tdx` defines no `tdx`-specific `fill` kinds.

PMI deliberately does not generate a TD HOB; platform description is delivered
through the cross-target [`dtb`](dtb.md) devicetree instead, which the PMI
consumer takes TDVF's role in consuming. For why PMI rejects the HOB, see
[Motivation §2](motivation.md#2-portable-safe-platform-definition-and-attestation).

## Status

The TDX target binding is a working draft. Open items:

- A reference PMI consumer for TDX (out of spec scope, but needed for the binding
  to be usable in practice). Expected responsibilities: reset-vector occupation,
  vCPU rendezvous, lazy memory acceptance, MMIO handling via
  `TDG.VP.VMCALL<#VE.RequestMMIO>`, CPUID page consumption, and kernel handoff.
  Upper-layer responsibilities (platform-metadata inspection, host-data merge,
  consumer validation) are layered on top of the base PMI consumer.
- The exact CDDL constraint on PE section `VirtualAddress` for the
  reset-vector-occupying load — whether the spec should mandate the architectural
  reset vector address or leave it to the consumer's metadata.
- Whether RTMR runtime extensions need image-side declaration; the working
  assumption is no — RTMRs are extended at runtime by the guest (the PMI consumer
  or the kernel), not at launch.
