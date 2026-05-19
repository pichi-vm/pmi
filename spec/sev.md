# `sev` Target

The `sev` target is built on [`vm`](vm.md). It inherits vm's base
launch model and `dtbo` action, extends vm's [`load`](vm.md#load-action)
action with SEV-SNP measurement semantics, and replaces vm's
[`vcpu`](vm.md#vcpu-field) field with a `sev:vmsa` action for the BSP
register state. The schema adds optional `id-block`/`id-auth` fields
for a signed launch and action types for the secrets and CPUID pages
SEV-SNP requires.

## PE section

A VMM targeting `sev` reads the `.pmi.sev` PE section. The section MUST
be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is absent,
the image does not support `sev` and the VMM MUST refuse to launch.

## Schema

```cddl
sev = {
  "version"    => uint,                  ; schema version (1)
  "dtb"        => tstr,                  ; PE section name; see dtb.md
  ? "id-block" => tstr,                  ; PE section: 96-byte SEV ID block
  ? "id-auth"  => tstr,                  ; PE section: SEV ID auth info (~4 KiB)
  "actions"    => [+ sev-action],        ; ordered launch recipe (step 4)
}

sev-action = sev-load / dtbo
           / sev-vmsa / sev-secrets / sev-cpuid
```

VMMs MUST reject sections with an unrecognized `version`, an unknown
top-level key, or an unknown action `type` value.

`id-block` and `id-auth` MUST both be present or both absent.

## Launch model

The `sev` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with the following SEV-SNP behavior layered on:

| Step          | API                  | Inputs                                                              |
| ------------- | -------------------- | ------------------------------------------------------------------- |
| 3. Initialize | `SNP_LAUNCH_START`   | host-supplied launch policy (see [Launch policy](#launch-policy))   |
| 4. Update     | `SNP_LAUNCH_UPDATE`  | each action in array order; `page_type` determined by action type   |
| 5. Finalize   | `SNP_LAUNCH_FINISH`  | `id-block` + `id-auth` (if present); `host_data` is deployer-supplied |

Within each step-4 action's PE section the VMM loads pages from the
lowest GPA to the highest, so the launch digest is deterministic for a
given action ordering.

## Launch policy

The launch policy passed to `SNP_LAUNCH_START` is **host-supplied** —
the VMM accepts it via VMM-defined input (CLI flag, config file, etc.),
which is out of scope for PMI. The format is the 64-bit POLICY field as
defined in the AMD SEV-SNP firmware ABI.

If `id-block` is present, the host launch policy MUST be compatible
with the policy field embedded in the signed ID block; the VMM MUST
verify and refuse to launch on mismatch.

If `id-block` is absent, the host has unconstrained latitude over the
launch policy.

The launch policy is not measured; it appears in the attestation report
for remote verification. A remote verifier MUST check policy fields in
the attestation report — the launch digest alone does not establish
policy properties.

## `id-block` and `id-auth`

The `id-block` and `id-auth` top-level fields name PE sections
containing, respectively, the 96-byte SEV ID block and the ~4 KiB ID
auth info (ECDSA P-384 signatures over the ID block). The VMM passes
these to `SNP_LAUNCH_FINISH` as `id_block` and `id_auth` at step 5.

Both PE sections MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`):

- The `id-block` PE section MUST have `VirtualSize == 96` and contain
  exactly the 96 bytes the AMD SEV-SNP ABI defines for the ID block.
- The `id-auth` PE section MUST have `VirtualSize == 4096` and contain
  the ID auth info structure defined by the same ABI.

Both fields are optional but linked: they MUST both be present or both
absent. An unsigned launch omits both; a signed launch carries both.

## `load` (extension)

`sev` extends the [base `load` action](vm.md#load-action) with one
optional field:

```cddl
sev-load = {
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

## SEV-specific actions

### `sev:vmsa`

Consumed at step 4 via `SNP_LAUNCH_UPDATE` with `page_type=vmsa`.

```cddl
sev-vmsa = {
  "type"    => "sev:vmsa",
  "section" => tstr,                ; PE section: 4 KiB VMSA page
}
```

The referenced PE section MUST be non-loaded
(`IMAGE_SCN_MEM_DISCARDABLE`) and have `VirtualSize == 4096`. Its
contents are the VMPL0 BSP register state at launch; the page is
measured with its actual content.

### `sev:secrets`

Consumed at step 4 via `SNP_LAUNCH_UPDATE` with `page_type=secrets`.

```cddl
sev-secrets = {
  "type"    => "sev:secrets",
  "section" => tstr,                ; PE section reserving the 4 KiB range
}
```

The referenced PE section MUST be a zero section (`SizeOfRawData == 0`,
`VirtualSize == 4096`). The PSP populates it with platform secrets at
launch. The page is measured with zero content in the launch digest
(the GPA is bound, the content is not).

### `sev:cpuid`

Consumed at step 4 via `SNP_LAUNCH_UPDATE` with `page_type=cpuid`.

```cddl
sev-cpuid = {
  "type"    => "sev:cpuid",
  "section" => tstr,                ; PE section reserving the 4 KiB range
}
```

The referenced PE section MUST be a zero section (`SizeOfRawData == 0`,
`VirtualSize == 4096`). The VMM fills the CPUID table with values it
wants to expose; the PSP validates entries and rejects impossible
values. The page is measured with zero content in the launch digest.
