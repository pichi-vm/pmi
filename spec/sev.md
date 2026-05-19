# `sev` Target

The `sev` target is built on [`vm`](vm.md). It inherits vm's base
launch model and `dtbo` action, extends vm's [`load`](vm.md#load-action)
action with SEV-SNP measurement semantics, and replaces vm's
[`vcpu`](vm.md#vcpu-field) field with a `vmsa` action for the BSP
register state. The schema adds an optional `id` field for signed
launches and `secrets` / `cpuid` actions for the SEV-SNP page-type
machinery.

## PE section

A VMM targeting `sev` reads the `.pmi.sev` PE section. The section MUST
be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is absent,
the image does not support `sev` and the VMM MUST refuse to launch.

## Schema

```cddl
sev = {
  "version" => uint,                     ; schema version (1)
  "dtb"     => tstr,                     ; PE section name; see dtb.md
  ? "id"    => id,                       ; signed launch identity; see id below
  "actions" => [+ sev-action],           ; ordered launch recipe (step 4)
}

id = {
  "block" => tstr,                       ; PE section: 96-byte SEV ID block
  "auth"  => tstr,                       ; PE section: SEV ID auth info (~4 KiB)
}

sev-action = load / dtbo / vmsa / secrets / cpuid
```

VMMs MUST reject sections with an unrecognized `version`, an unknown
top-level key, or an unknown action `type` value.

## Launch model

The `sev` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with the following SEV-SNP behavior layered on:

| Step          | API                  | Inputs                                                              |
| ------------- | -------------------- | ------------------------------------------------------------------- |
| 3. Initialize | `SNP_LAUNCH_START`   | host-supplied launch policy (see [Launch policy](#launch-policy))   |
| 4. Update     | `SNP_LAUNCH_UPDATE`  | each action in array order; `page_type` determined by action type   |
| 5. Finalize   | `SNP_LAUNCH_FINISH`  | `id.block` + `id.auth` (if `id` is present); `host_data` is deployer-supplied |

Within each step-4 action's PE section the VMM loads pages from the
lowest GPA to the highest, so the launch digest is deterministic for a
given action ordering.

## Launch policy

The launch policy passed to `SNP_LAUNCH_START` is **host-supplied** —
the VMM accepts it via VMM-defined input (CLI flag, config file, etc.),
which is out of scope for PMI. The format is the 64-bit POLICY field as
defined in the AMD SEV-SNP firmware ABI.

If `id` is present, the host launch policy MUST be compatible with the
policy field embedded in the signed ID block; the VMM MUST verify and
refuse to launch on mismatch.

If `id` is absent, the host has unconstrained latitude over the launch
policy.

The launch policy is not measured; it appears in the attestation report
for remote verification. A remote verifier MUST check policy fields in
the attestation report — the launch digest alone does not establish
policy properties.

## `id` field

The optional `id` field carries a signed launch identity — present on
signed launches, absent on unsigned ones. It names two PE sections:

```cddl
id = {
  "block" => tstr,                  ; PE section: 96-byte SEV ID block
  "auth"  => tstr,                  ; PE section: SEV ID auth info (~4 KiB)
}
```

The VMM passes the two sections to `SNP_LAUNCH_FINISH` as `id_block`
and `id_auth` at step 5.

Both PE sections MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`):

- The `block` PE section MUST have `VirtualSize == 96` and contain
  exactly the 96 bytes the AMD SEV-SNP ABI defines for the ID block.
- The `auth` PE section MUST have `VirtualSize == 4096` and contain
  the ID auth info structure defined by the same ABI (ECDSA P-384
  signatures over the ID block, plus the ID key and optional author
  key).

Pairing is structural: when `id` is present, both `block` and `auth`
keys are required.

## `load` action

`sev` extends the [base `load` action](vm.md#load-action) with one
optional field:

```cddl
load = {
  "type"        => "load",
  "section"     => tstr,                ; PE section name to load
  ? "measured"  => bool,                ; default true
}
```

Consumed at step 4 via `SNP_LAUNCH_UPDATE`. When `measured` is `true`
(the default), the loaded bytes contribute to the launch digest;
on-disk bytes are measured as normal data pages, and zero-filled bytes
are measured via `page_type=zero` (which validates pages as zero
without transferring data and produces a different measurement than
loading actual zeros as data pages). VMM implementations MUST NOT
substitute data-page loads for zero-page operations or vice versa.
When `measured` is `false`, the bytes are still loaded into guest
memory but are not fed to the measurement API — used for VMM-supplied
data the verifier does not need to bind to.

## `dtbo` action

Same as the [base `dtbo` action](vm.md#dtbo-action) defined by `vm`,
without modification. The overlay is not fed to `SNP_LAUNCH_UPDATE`
and does not contribute to the launch digest.

## `vmsa` action

```cddl
vmsa = {
  "type"    => "vmsa",
  "section" => tstr,                ; PE section: 4 KiB VMSA page
}
```

The referenced PE section MUST be non-loaded
(`IMAGE_SCN_MEM_DISCARDABLE`) and have `VirtualSize == 4096`. Its
contents are the VMPL0 BSP register state at launch, in the layout
defined by the AMD SEV-SNP firmware ABI.

At step 4 the VMM:

1. Reads the PE section's 4 KiB contents from `PointerToRawData`.
2. Calls `SNP_LAUNCH_UPDATE` with `page_type=vmsa`, supplying those
   contents and targeting the section's `VirtualAddress` in guest
   memory.

The PSP installs the VMSA at the named GPA. The page is measured with
its actual content, so the launch digest binds the BSP's initial
register state.

## `secrets` action

```cddl
secrets = {
  "type"    => "secrets",
  "section" => tstr,                ; PE section reserving the 4 KiB range
}
```

The referenced PE section MUST be a zero section
(`SizeOfRawData == 0`, `VirtualSize == 4096`) — it reserves a GPA
range with no on-disk data for a page the PSP will populate at
launch.

At step 4 the VMM calls `SNP_LAUNCH_UPDATE` with `page_type=secrets`,
targeting the section's `VirtualAddress` in guest memory. No content
is supplied; the PSP populates the page with platform secrets in
encrypted guest memory.

The page is measured as a zero page in the launch digest — the GPA
and page type are bound, the content is not (the secrets aren't
knowable to a verifier ahead of time).

## `cpuid` action

```cddl
cpuid = {
  "type"    => "cpuid",
  "section" => tstr,                ; PE section reserving the 4 KiB range
}
```

The referenced PE section MUST be a zero section
(`SizeOfRawData == 0`, `VirtualSize == 4096`) — it reserves a GPA
range with no on-disk data for the CPUID table the VMM will provide
at launch.

At step 4 the VMM:

1. Constructs the CPUID table it wants to expose to the guest, in
   the layout defined by the AMD SEV-SNP firmware ABI.
2. Calls `SNP_LAUNCH_UPDATE` with `page_type=cpuid`, supplying that
   table and targeting the section's `VirtualAddress` in guest
   memory.

The PSP validates each CPUID entry against the actual processor's
capabilities and rejects entries that claim functionality the
processor does not support. The page is measured as a zero page in
the launch digest — the GPA and page type are bound, the content is
not.
