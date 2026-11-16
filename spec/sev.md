# `sev` Extension

**Prefix:** `sev`.

The `sev` extension provides the essential functionality for launching a PMI as
a virtual machine on AMD SEV (3.0+; a.k.a. SEV-SNP). It defines five extension
points:

1. The new target [`.pmi.sev`](#1-new-target-pmisev).
2. The new target attribute [`sev:id`](#2-new-target-attribute-sevid).
3. The new `fill` kind [`sev:secrets`](#3-new-fill-kind-sevsecrets).
4. The new `fill` kind [`sev:cpuid`](#4-new-fill-kind-sevcpuid).
5. The new target attribute
   [`sev:features`](#5-new-target-attribute-sevfeatures).

## 1. New target: `.pmi.sev`

The `.pmi.sev` PE section carries the `sev` target spec, subject to the
[page granularity](granularity.md) rules.

### Launch model

The `sev` target follows the [core launch model](core.md#launch-model), layering
the SEV-SNP firmware ABI onto it in six ordered steps:

1. Read the `.pmi.sev` PE section.
2. `SNP_LAUNCH_START` with the host-supplied launch policy (see
   [Launch policy](#launch-policy)). If `sev:features` sets `SecureTSC`, also
   pass the host-supplied TSC frequency (see
   [§5](#5-new-target-attribute-sevfeatures)).
3. Process each entry in `actions` in array order using the [`load`](#load),
   [`fill`](#fill), and target-specific kind operations defined below.
4. Submit the BSP's VMSA (see [Boot vCPU](#boot-vcpu)).
5. `SNP_LAUNCH_FINISH`, passing `sev:id.block` + `sev:id.auth` if `sev:id` is
   present, plus the deployer-supplied `host_data` (see
   [Launch policy](#launch-policy)).
6. Start the guest.

### Keys

The `.pmi.sev` CBOR map follows the [core target shape](core.md#shape). Its
`version` MUST be `1`. It adds the following keys:

- **`cpu:profile`**: vCPU ISA baseline (required; see [cpu.md](cpu.md)).
- **`sev:features`**: SEV-SNP features to enable (optional; see
  [§5](#5-new-target-attribute-sevfeatures)).
- **`sev:id`**: signed launch identity (optional; see
  [§2](#2-new-target-attribute-sevid)).

### Validation

The [core validation rules](core.md#validation) apply. The `sev` target adds the
`sev:id`-pairing rule described under [§2](#2-new-target-attribute-sevid) and
the `sev:features` rules described under
[§5](#5-new-target-attribute-sevfeatures).

The VMM MUST also refuse to launch if an action's `[gpa, gpa + VirtualSize)`
range intersects the VMSA page `[0xFFFFFFFFF000, 0x1000000000000)` (see
[Boot vCPU](#boot-vcpu)).

### Launch policy

The launch policy passed to `SNP_LAUNCH_START` is host-supplied. The VMM
accepts it via VMM-defined input (CLI flag, config file, etc.), which is out of
scope for PMI. The format is the 64-bit POLICY field as defined in the AMD
SEV-SNP firmware ABI.

If `sev:id` is present, the host launch policy must be compatible with the
policy field embedded in the signed ID block. This is enforced by the PSP
firmware at `SNP_LAUNCH_FINISH`, which verifies the signed ID block against the
launch and fails the launch on mismatch; the VMM need not check it.

If `sev:id` is absent, the host has unconstrained latitude over the launch
policy.

The launch policy is not measured; it appears in the attestation report for
remote verification. A remote verifier MUST check policy fields in the
attestation report, since the launch digest alone does not establish policy
properties.

The deployer also supplies a 32-byte `host_data` value to `SNP_LAUNCH_FINISH`.
Like the launch policy, it is host-supplied, not carried by PMI, and unmeasured;
the firmware reflects it verbatim in the attestation report for the verifier.

### `load`

On `sev`, the `default` kind submits the section's pages via
`SNP_LAUNCH_UPDATE`: data pages of a Data or Padded section as
`PAGE_TYPE_NORMAL` (measured into the launch digest), and the zero-fill tail of
a Padded section or all of a Zero section as `PAGE_TYPE_ZERO` (validated as zero
without transferring data, yielding a different measurement than loading actual
zeros). The VMM MUST NOT substitute data-page operations for zero-page
operations or vice versa.

### `fill`

For the core [fill classes](core.md#measurement), the VMM MUST submit every
occupied page through `SNP_LAUNCH_UPDATE` into private guest memory:

- **Measured fills:** use `PAGE_TYPE_NORMAL`, including for zero-filled pages.
  The PSP incorporates content, GPA, and page type into the launch digest.
- **Unmeasured fills:** use `PAGE_TYPE_UNMEASURED`. The PSP binds the GPA and
  page type while excluding the content.

Target-specific kinds [`sev:secrets`](#3-new-fill-kind-sevsecrets) and
[`sev:cpuid`](#4-new-fill-kind-sevcpuid) use their dedicated page types.

### Measurement

The PSP computes the launch digest from 4 KiB `PAGE_INFO` records. Each record
binds the page's GPA, page type, and content as specified by that type. A 2 MiB
submission produces the same digest as the 512 corresponding 4 KiB submissions.
The VMM MAY choose the submission page size and MUST follow the core's
[measurement order](core.md#measurement-determinism). Nothing is measured before
the actions. The BSP's VMSA is the only thing measured after them (see
[Launch model](#launch-model)).

### Boot vCPU

The BSP starts in the [x86-64 boot vCPU state](x86-64.md#boot-vcpu), except that
`EFER.SVME` is set, as `VMRUN` requires. The VMM builds the BSP's VMSA from that
state. The VMSA layout and field names come from the AMD64 Architecture
Programmer's Manual (APM), Volume 2. Each register the state lists goes in the
VMSA field with the same name in upper case, except that `IA32_PAT` goes in
`G_PAT` and `FCW` goes in `X87_FCW`. The table below gives the encoded segment
attributes, the encoded x87 tag word, and every byte the state does not list.
Each `attrib` value uses the APM's 12-bit encoding of the segment attributes.
Together, the state and the table fix every VMSA byte, so every compliant VMM
produces the same bytes.

| VMSA field                                                      | Value                                                 |
| --------------------------------------------------------------- | ----------------------------------------------------- |
| `CS.attrib`                                                     | `0x0C9B`                                              |
| `DS.attrib`, `ES.attrib`, `FS.attrib`, `GS.attrib`, `SS.attrib` | `0x0C93`                                              |
| `TR.attrib`                                                     | `0x008B`                                              |
| `X87_FTW`                                                       | `0`                                                   |
| `SEV_FEATURES`                                                  | `SNPActive` (bit 0) plus the bits `sev:features` sets |
| every other byte                                                | `0`                                                   |

The VMM MUST submit exactly one VMSA, the BSP's, via `SNP_LAUNCH_UPDATE` with
`PAGE_TYPE_VMSA` at the fixed GPA `0xFFFFFFFFF000`. The PSP measures the page's
GPA, page type, and content, so the launch digest binds the BSP's initial
register state. No action places the VMSA, so this spec fixes its GPA.

### `cpu:profile`

The VMM populates the CPUID page (via the
[`sev:cpuid`](#4-new-fill-kind-sevcpuid) fill kind) with entries that advertise
at least the profile. The PSP validates each entry against the actual processor
and rejects entries claiming functionality the processor does not support. The
launch digest binds the page type and GPA, not the content; `cpu:profile` does
not enter the launch digest, and the VMM MAY include host-supported features
beyond the profile in the CPUID page.

Leaving the CPUID page unmeasured is safe: the PSP rejects entries claiming
features the processor lacks (no over-claim), and the SEV feature set
(`SEV_FEATURES`) lives in the measured VMSA. The only host deviation left is
under-provisioning, a denial of service (see [Measured vs. host-controlled
inputs](core.md#measured-vs-host-controlled-inputs)).

## 2. New target attribute: `sev:id`

The optional `sev:id` field carries a signed launch identity. It is present on
signed launches and absent on unsigned ones. It names two PE sections:

```cddl
sev-id = {
  "block" => tstr,                  ; PE section: 96-byte SEV ID block
  "auth"  => tstr,                  ; PE section: SEV ID auth info (~4 KiB)
}
```

The VMM passes the two sections to `SNP_LAUNCH_FINISH` as `id_block` and
`id_auth` at step 5.

Both PE sections MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). They are not
loaded into guest memory; the VMM reads them from the file and copies them into
the `SNP_LAUNCH_FINISH` command. `VirtualAddress` is unconstrained, since these
sections are never placed in guest memory, and `PointerToRawData` MUST be
4K-aligned so the VMM can mmap each section directly from the file.

- The `block` PE section MUST have `VirtualSize == 96` and
  `SizeOfRawData == 96`, and contain exactly the 96 bytes the AMD SEV-SNP ABI
  defines for the ID block.

- The `auth` PE section MUST have `VirtualSize == 4096` and
  `SizeOfRawData == 4096`, and contain the ID auth info structure defined by the
  same ABI (ECDSA P-384 signatures over the ID block, plus the ID key and
  optional author key).

Pairing is structural: when `sev:id` is present, both `block` and `auth` keys
MUST be present; the VMM MUST refuse to launch on a spec that contains only one.

## 3. New `fill` kind: `sev:secrets`

The VMM submits the page via `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_SECRETS`. No
content is supplied; the PSP populates the page with platform secrets in
encrypted guest memory at launch. The referenced PE section MUST be a Zero
section (`SizeOfRawData == 0`) with `VirtualSize == 4096`. The page contributes
to the launch digest as a typed page: the GPA and page type are bound, the
content is not.

## 4. New `fill` kind: `sev:cpuid`

The VMM constructs the CPUID table it wants to expose to the guest in the layout
defined by the AMD SEV-SNP firmware ABI, then submits the table via
`SNP_LAUNCH_UPDATE` with `PAGE_TYPE_CPUID`. The PSP validates each CPUID entry
against the actual processor's capabilities and rejects entries that claim
functionality the processor does not support. The referenced PE section MUST be
a Zero section (`SizeOfRawData == 0`) with `VirtualSize == 4096`. The page
contributes to the launch digest as a typed page: the GPA and page type are
bound, the content is not.

## 5. New target attribute: `sev:features`

`sev:features` selects the SEV-SNP features the BSP's VMSA enables beyond
`SNPActive` (see [Boot vCPU](#boot-vcpu)). It is an unsigned integer. Each bit
has the meaning it has in the VMSA's `SEV_FEATURES` field. When `sev:features`
is absent, no feature is selected.

```cddl
sev-features = uint                 ; bits from the table below
```

| Bit | Feature               |
| --- | --------------------- |
| 5   | `DebugVirtualization` |
| 9   | `SecureTSC`           |

The VMM MUST refuse to launch if `sev:features` sets a bit the table does not
list. It MUST also refuse to launch if the host cannot enable a feature that
`sev:features` sets.

When `sev:features` sets `SecureTSC`, the VMM passes a host-supplied TSC
frequency to `SNP_LAUNCH_START`. This value is not measured. A bad value can
only make the guest's clock run fast or slow. The host already controls the
guest's wall-clock time. A guest cannot rely on its clock for security. The
value can therefore cause at most denial of service (see [Measured vs.
host-controlled inputs](core.md#measured-vs-host-controlled-inputs)).

## Example

A `.pmi.sev` that launches OVMF and a kernel under a signed identity, with
secrets and CPUID pages:

```cbor-diag
{
  "version": 1,
  "cpu:profile": "x86-64-v3",
  "sev:id": {"block": ".sev.id.block", "auth": ".sev.id.auth"},
  "dt:dtb": ".dtb",
  "actions": [
    {"type": "load", "gpa": 0xFFC00000, "section": ".ovmf"},
    {"type": "load", "gpa": 0x1000000,  "section": ".linux"},
    {"type": "load", "gpa": 0x4000000,  "section": ".initrd"},
    {"type": "load", "gpa": 0x2000000,  "section": ".cmdline"},
    {"type": "load", "gpa": 0x2001000,  "section": ".dtb"},
    {"type": "fill", "gpa": 0x2011000,  "section": ".dtbo", "kind": "dt:dtbo"},
    {"type": "fill", "gpa": 0x2021000,  "section": ".sev.secrets", "kind": "sev:secrets"},
    {"type": "fill", "gpa": 0x2022000,  "section": ".sev.cpuid", "kind": "sev:cpuid"}
  ]
}
```

The `default` loads submit `PAGE_TYPE_NORMAL` pages
(including the base `.dtb`); `.dtbo` goes in unmeasured for the guest to
validate and merge; `.sev.secrets` and `.sev.cpuid` submit `PAGE_TYPE_SECRETS`
and `PAGE_TYPE_CPUID`. Then the VMM submits the BSP's VMSA with `PAGE_TYPE_VMSA`
(see [Boot vCPU](#boot-vcpu)). `SNP_LAUNCH_FINISH` passes `id_block` and
`id_auth` from `.sev.id.block` / `.sev.id.auth`. The BSP starts at the reset
vector inside `.ovmf`, and OVMF boots the kernel.
