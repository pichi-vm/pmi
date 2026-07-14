# `tdx` Extension

**Prefix:** `tdx`.

The `tdx` extension provides the essential functionality for launching a PMI as
a confidential virtual machine on Intel TDX. It defines one extension point:

1. The new target [`.pmi.tdx`](#1-new-target-pmitdx).

## 1. New target: `.pmi.tdx`

The `.pmi.tdx` PE section carries the `tdx` target spec, subject to the
[page granularity](granularity.md) rules.

### Launch model

The `tdx` target follows the [core launch model](core.md#launch-model), layering
the Intel TDX firmware ABI onto the five ordered steps:

1. Read the `.pmi.tdx` PE section.
2. `KVM_TDX_INIT_VM` then `KVM_TDX_INIT_VCPU` with the host-supplied TD
   parameters (see [TD parameters](#td-parameters)).
3. Process each entry in `actions` in array order via `KVM_TDX_INIT_MEM_REGION`;
   the `KVM_TDX_MEASURE_MEMORY_REGION` flag is set per the action's kind.
4. `KVM_TDX_FINALIZE_VM`, which locks MRTD.
5. Start the guest.

MRTD extension is reproducible from the image bytes per the order fixed by
[Measurement determinism](core.md#measurement-determinism) and the per-page
sub-operation order defined under [`load`](#load) below.

### Keys

The `.pmi.tdx` CBOR map follows the [core target shape](core.md#shape). Its
`version` MUST be `1`. It adds one required key:

- **`cpu:profile`**: vCPU ISA baseline (see [cpu.md](cpu.md)).

### Validation

The [core validation rules](core.md#validation) apply. The `tdx` target adds no
further validation rules.

### TD parameters

`TD_PARAMS` (including `ATTRIBUTES`, `XFAM`, CPUID configuration, and the
`MRCONFIGID` / `MROWNER` / `MROWNERCONFIG` deployer fields) is host-supplied.
The VMM passes it to `KVM_TDX_INIT_VM`; PMI does not carry it. None of it
enters MRTD, which is built only from the pages added by `load` actions, so the
host cannot perturb the image measurement through `TD_PARAMS`. Each field is
attested in its own report field; a remote verifier MUST check those separately,
as it does for SEV's launch policy.

### `load`

On `tdx`, the `default` kind submits the section's pages via
`KVM_TDX_INIT_MEM_REGION` with `KVM_TDX_MEASURE_MEMORY_REGION` set. MRTD is
computed at the TDX module's fixed granularity (4 KiB for `TDH.MEM.PAGE.ADD`,
binding the page GPA, and 256 bytes for `TDH.MR.EXTEND`, binding content),
independent of the page size the VMM uses to back or map guest memory.

The submission order is fully pinned, so MRTD is reproducible from the image
bytes: sections in `actions` array order; within a section, pages in ascending
GPA order; for each page, `TDH.MEM.PAGE.ADD` first, then the sixteen
`TDH.MR.EXTEND` operations over that page's 256-byte chunks in ascending offset
order (0, 256, …, 3840), before advancing to the next page. The VMM MUST NOT
batch all `TDH.MEM.PAGE.ADD` operations ahead of the `TDH.MR.EXTEND` operations.

TDX starts the boot vCPU at the architectural reset vector with its initial
register state fixed by the TDX module. PMI provides no mechanism to set initial
register contents on TDX; the `tdx` target defines no `vm:vcpu`/`vcpu-x64`. A
compliant VMM MUST set to zero any initial register value it can influence,
notably R8, which the module mirrors into RCX. This state does not enter MRTD and
is not attestable, so the image MUST carry a measured PMI consumer, loaded at
the reset vector via a `default` load (and thus part of MRTD), that establishes
boot state itself: it obtains platform facts (GPAW, vCPU index, attributes) from
`TDCALL[TDG.VP.INFO]` and MUST NOT rely on the initial register contents,
including R8/RCX. The consumer performs vCPU rendezvous and hands off to the
kernel; its implementation is out of scope for this spec.

### `fill`

`tdx` defines no `tdx`-specific `fill` kinds.

PMI deliberately does not generate a TD HOB; platform description is delivered
through the [`dt:dtbo`](dt.md) fill kind instead, which the PMI
consumer takes TDVF's role in consuming. For why PMI rejects the HOB, see
[Motivation §2](motivation.md#2-portable-safe-platform-definition-and-attestation).

### `cpu:profile`

The VMM builds `XFAM` and `CPUID_VALUES` in `TD_PARAMS` from the profile and
passes them to `KVM_TDX_INIT_VM`. `TD_PARAMS` does not enter MRTD; the VMM MAY
configure `XFAM` and `CPUID_VALUES` to expose host-supported features beyond
the profile. The exposed `XFAM` and TD attributes are reflected in the TD
report (`tdx_xfam` and `tdx_td_attributes`) for verifier policy. The TDX module
enforces certain "fixed-1" CPUID bits that the VMM cannot disable; those are
exposed regardless of profile and remain visible in the report fields.

Leaving `TD_PARAMS` unmeasured is safe: the TDX module validates `CPUID_VALUES`
against the hardware and enforces the fixed-1 bits (no over-claim), and `XFAM`
and the TD attributes are attested in the report for the verifier. The only
host deviation is under-provisioning, a denial of service (see [Measured vs.
host-controlled inputs](core.md#measured-vs-host-controlled-inputs)).

## Example

A `.pmi.tdx` carrying the PMI consumer at the reset vector, the kernel payload,
and a host devicetree:

```cbor-diag
{
  "version": 1,
  "cpu:profile": "x86-64-v4",
  "dt:dtb": ".dtb",
  "actions": [
    {"type": "load", "gpa": 0xFFFF0000, "section": ".tdx.consumer"},
    {"type": "load", "gpa": 0x1000000,  "section": ".linux"},
    {"type": "load", "gpa": 0x4000000,  "section": ".initrd"},
    {"type": "load", "gpa": 0x2000000,  "section": ".cmdline"},
    {"type": "load", "gpa": 0x2001000,  "section": ".dtb"},
    {"type": "fill", "gpa": 0x2011000,  "section": ".dtbo", "kind": "dt:dtbo"}
  ]
}
```

After `KVM_TDX_INIT_VM` / `KVM_TDX_INIT_VCPU` with the host-supplied TD
parameters, each `default` load is submitted via `KVM_TDX_INIT_MEM_REGION`
with the measure flag set, so `.tdx.consumer` (the PMI consumer), `.linux`,
`.initrd`, `.cmdline`, and the base `.dtb` all extend MRTD. The `.dtbo` is
placed as an unmeasured page for the consumer to validate and merge.
`KVM_TDX_FINALIZE_VM` locks MRTD; the consumer runs at the reset vector,
validates and consumes the devicetree, and hands off to the kernel.
