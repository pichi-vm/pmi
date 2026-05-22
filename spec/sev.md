# `sev` Target

The `sev` target is built on [`vm`](vm.md). It inherits vm's base
launch model, extends vm's [`load`](vm.md#load-action) and
[`fill`](vm.md#fill-action) actions with SEV-SNP-specific kinds, and
replaces vm's [`vcpu`](vm.md#vcpu-field) field with a `vmsa` load
kind for the BSP register state. The schema adds an optional `id`
field for signed launches.

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

sev-action = load / fill
```

The schema-strictness and action-array validation rules from
[`vm`](vm.md#schema) apply: unrecognized `version`, unknown key in
any defined CBOR map, unknown action `type`, unknown action `kind`,
non-existent section reference, duplicate section reference, and
overlapping `[VirtualAddress, VirtualAddress + VirtualSize)` ranges
all cause the VMM to refuse to launch.

## Launch model

The `sev` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with the following SEV-SNP behavior layered on:

| Step          | API                  | Inputs                                                              |
| ------------- | -------------------- | ------------------------------------------------------------------- |
| 3. Initialize | `SNP_LAUNCH_START`   | host-supplied launch policy (see [Launch policy](#launch-policy))   |
| 4. Update     | per action kind      | each action in array order; firmware path and `page_type` derive from the action's kind (see per-kind sections below) |
| 5. Finalize   | `SNP_LAUNCH_FINISH`  | `id.block` + `id.auth` (if `id` is present); `host_data` is deployer-supplied |

Within each step-4 action's PE section the VMM submits pages from the
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

### Measurement scope

Per [Measurement determinism](overview.md#measurement-determinism),
the SEV-SNP launch digest is a deterministic function of the PMI
image bytes alone: every page that contributes to the digest is named
by an action in the spec's `actions` array, and within each action's
PE section the VMM submits pages in GPA order. The host launch policy
is the only value the SEV-SNP architecture surfaces to the verifier
that is not part of the launch digest; image authors who require
policy reproducibility in attestation MUST include the `id` block,
which binds the host policy under the signed ID key.

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
keys MUST be present; the VMM MUST refuse to launch on a spec that
contains only one.

## `load` action

`sev` extends the [base `load` action](vm.md#load-action) with two
additional kinds; the default kind for sev's load is `measured`.

### Schema

```cddl
load = {
  "type"    => "load",
  "section" => tstr,                ; PE section name to load
  ? "kind"  => "measured" / "unmeasured" / "vmsa",  ; default "measured"
}
```

### kind `measured`

The default kind. The VMM submits the PE section's pages via
`SNP_LAUNCH_UPDATE`:

- For the data portion of a Data or Padded section, `PAGE_TYPE_NORMAL`
  (the loaded bytes contribute to the launch digest).
- For the zero-fill tail of a Padded section or all of a Zero section,
  `PAGE_TYPE_ZERO` (the page is validated as zero without transferring
  data and produces a different measurement than loading actual zeros
  as data pages).

VMM implementations MUST NOT substitute data-page operations for
zero-page operations or vice versa.

### kind `unmeasured`

The VMM submits the PE section's pages via `SNP_LAUNCH_UPDATE` with
`PAGE_TYPE_UNMEASURED`. The bytes land in guest memory but do not
contribute to the launch digest. Used for VMM-supplied data the
verifier does not need to bind to.

### kind `vmsa`

The VMM submits the PE section's 4 KiB contents via
`SNP_LAUNCH_UPDATE` with `PAGE_TYPE_VMSA`. The section's contents are
the VMPL0 BSP register state at launch, in the layout defined by the
AMD SEV-SNP firmware ABI. The PSP installs the VMSA at the named GPA.
The page is measured with its actual content, so the launch digest
binds the BSP's initial register state.

The referenced PE section MUST have `VirtualSize == 4096`.

## `fill` action

`sev` extends the [base `fill` action](vm.md#fill-action) with two
additional kinds.

### Schema

```cddl
fill = {
  "type"    => "fill",
  "section" => tstr,                ; zero PE section to populate
  "kind"    => "dtbo" / "secrets" / "cpuid",
}
```

### kind `dtbo`

Same as the [base `dtbo` fill kind](vm.md#kind-dtbo). The VMM
generates the overlay and writes it to the section's GPA range; the
page bypasses `SNP_LAUNCH_UPDATE` and does not contribute to the
launch digest. See [`dtbo` overlay](vm.md#dtbo-overlay) for content
and consumer-validation rules.

### kind `secrets`

The VMM submits the page via `SNP_LAUNCH_UPDATE` with
`PAGE_TYPE_SECRETS`. No content is supplied; the PSP populates the
page with platform secrets in encrypted guest memory at launch.

The referenced PE section MUST have `VirtualSize == 4096`. The page
contributes to the launch digest as a typed page — the GPA and page
type are bound, the content is not (the secrets aren't knowable to a
verifier ahead of time).

### kind `cpuid`

The VMM constructs the CPUID table it wants to expose to the guest in
the layout defined by the AMD SEV-SNP firmware ABI, then submits the
table via `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_CPUID`. The PSP
validates each CPUID entry against the actual processor's
capabilities and rejects entries that claim functionality the
processor does not support.

The referenced PE section MUST have `VirtualSize == 4096`. The page
contributes to the launch digest as a typed page — the GPA and page
type are bound, the content is not.
