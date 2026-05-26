# `sev` Extension

**Prefix:** `sev`.

The `sev` extension provides the essential functionality for launching a PMI as
a virtual machine on AMD SEV (3.0+; a.k.a. SEV-SNP). It defines five extension
points:

1. The new target [`.pmi.sev`](#1-new-target-pmisev).
2. The new target attribute [`sev:id`](#2-new-target-attribute-sevid).
3. The new `load` kind [`sev:vmsa`](#3-new-load-kind-sevvmsa).
4. The new `fill` kind [`sev:secrets`](#4-new-fill-kind-sevsecrets).
5. The new `fill` kind [`sev:cpuid`](#5-new-fill-kind-sevcpuid).

## 1. New target: `.pmi.sev`

The `.pmi.sev` PE section carries the `sev` target spec, subject to the
[core PE constraints](constraints.md#pe-constraints).

### Launch model

The `sev` target follows the [base launch model](vm.md#launch-model) defined by
`vm`, layering the SEV-SNP firmware ABI onto the five ordered steps:

1. Read the `.pmi.sev` PE section.
2. `SNP_LAUNCH_START` with the host-supplied launch policy (see
   [Launch policy](#launch-policy)).
3. Process each entry in `actions` in array order; the firmware path and
   `PAGE_TYPE` derive from each action's kind (see the kind sections below).
4. `SNP_LAUNCH_FINISH`, passing `sev:id.block` + `sev:id.auth` if `sev:id` is
   present, plus the deployer-supplied `host_data` (see
   [Launch policy](#launch-policy)).
5. Start the guest.

### Keys

The `.pmi.sev` CBOR map follows the [core target shape](core.md#shape). Its
`version` MUST be `1`. It adds one optional key:

- **`sev:id`** — signed launch identity (see
  [§2](#2-new-target-attribute-sevid)).

### Validation

The [core validation rules](core.md#validation) apply. The `sev` target adds the
`sev:id`-pairing rule described under [§2](#2-new-target-attribute-sevid).

### Launch policy

The launch policy passed to `SNP_LAUNCH_START` is **host-supplied** — the VMM
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
attestation report — the launch digest alone does not establish policy
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

## 2. New target attribute: `sev:id`

The optional `sev:id` field carries a signed launch identity — present on signed
launches, absent on unsigned ones. It names two PE sections:

```cddl
sev-id = {
  "block" => tstr,                  ; PE section: 96-byte SEV ID block
  "auth"  => tstr,                  ; PE section: SEV ID auth info (~4 KiB)
}
```

The VMM passes the two sections to `SNP_LAUNCH_FINISH` as `id_block` and
`id_auth` at step 4.

Both PE sections MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). They are not
loaded into guest memory; the VMM reads them from the file and copies them into
the `SNP_LAUNCH_FINISH` command. `PointerToRawData` MUST be 4K-aligned and
`SizeOfRawData` MUST be 4096 so the VMM can mmap each section directly from the
file. `VirtualAddress` is unconstrained: these sections are never placed in
guest memory.

- The `block` PE section MUST have `VirtualSize == 96` and contain exactly the
  96 bytes the AMD SEV-SNP ABI defines for the ID block.

- The `auth` PE section MUST have `VirtualSize == 4096` and contain the ID auth
  info structure defined by the same ABI (ECDSA P-384 signatures over the ID
  block, plus the ID key and optional author key).

Pairing is structural: when `sev:id` is present, both `block` and `auth` keys
MUST be present; the VMM MUST refuse to launch on a spec that contains only one.

## 3. New `load` kind: `sev:vmsa`

The VMM submits the PE section's 4 KiB contents via `SNP_LAUNCH_UPDATE` with
`PAGE_TYPE_VMSA`. The section's contents are the VMPL0 BSP register state at
launch, in the layout defined by the AMD SEV-SNP firmware ABI. The PSP installs
the VMSA at the named GPA. The page is measured with its actual content, so the
launch digest binds the BSP's initial register state. The VMSA is the 4096-byte
VM Save Area defined by the AMD SEV-SNP firmware ABI; the referenced PE section
MUST be a Data section (`SizeOfRawData == 4096`, `VirtualSize == 4096`).

## 4. New `fill` kind: `sev:secrets`

The VMM submits the page via `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_SECRETS`. No
content is supplied; the PSP populates the page with platform secrets in
encrypted guest memory at launch. The referenced PE section MUST be a Zero
section (`SizeOfRawData == 0`) with `VirtualSize == 4096`. The page contributes
to the launch digest as a typed page — the GPA and page type are bound, the
content is not.

## 5. New `fill` kind: `sev:cpuid`

The VMM constructs the CPUID table it wants to expose to the guest in the layout
defined by the AMD SEV-SNP firmware ABI, then submits the table via
`SNP_LAUNCH_UPDATE` with `PAGE_TYPE_CPUID`. The PSP validates each CPUID entry
against the actual processor's capabilities and rejects entries that claim
functionality the processor does not support. The referenced PE section MUST be
a Zero section (`SizeOfRawData == 0`) with `VirtualSize == 4096`. The page
contributes to the launch digest as a typed page — the GPA and page type are
bound, the content is not.

## Example

A `.pmi.sev` that launches a service module (SVSM) and OVMF under a signed
identity, with secrets and CPUID pages:

```cbor-diag
{
  "version": 1,
  "sev:id": {"block": ".sev.idblock", "auth": ".sev.idauth"},
  "actions": [
    {"type": "load", "section": ".svsm"},
    {"type": "load", "section": ".ovmf"},
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"},
    {"type": "fill", "section": ".dtb", "kind": "dtb"},
    {"type": "fill", "section": ".sev.secrets", "kind": "sev:secrets"},
    {"type": "fill", "section": ".sev.cpuid", "kind": "sev:cpuid"},
    {"type": "load", "section": ".sev.vmsa", "kind": "sev:vmsa"}
  ]
}
```

`SNP_LAUNCH_START` verifies the host policy against the policy embedded in the
signed `.sev.idblock`. The `default` loads submit `PAGE_TYPE_NORMAL` pages; `.dtb`
goes in unmeasured; `.sev.secrets`, `.sev.cpuid`, and `.sev.vmsa` submit
`PAGE_TYPE_SECRETS`, `PAGE_TYPE_CPUID`, and `PAGE_TYPE_VMSA`.
`SNP_LAUNCH_FINISH` passes `id_block` and `id_auth` from `.sev.idblock` /
`.sev.idauth`. The SVSM starts at VMPL0, initializes a vTPM, transitions OVMF to
VMPL1, and OVMF boots the kernel.
