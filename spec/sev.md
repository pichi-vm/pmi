# `sev` Target

The `sev` target is the AMD SEV 3.0 (SEV-SNP) launch path. The VMM reads the
image's base [DTB](dtb.md), processes the actions list to drive the SEV
launch APIs (`SNP_LAUNCH_START`, `SNP_LAUNCH_UPDATE`, `SNP_LAUNCH_FINISH`),
and starts the guest under SEV protection.

The `sev` target is built on [`vm`](vm.md). It inherits `vm`'s base
launch model and `vm`'s [`dtbo`](dtbo.md) action, extends `vm`'s
[`load`](vm.md#load-action) action with SEV-SNP measurement semantics
(see [`load` below](#load)), and adds its own launch-specific action
types. It does not use `vm`'s [`vcpu`](vm.md#vcpu) field; the SEV
equivalent is [`sev:vmsa`](#sevvmsa).

## PE section

A VMM targeting `sev` reads the `.pmi.sev` PE section. The section MUST be
non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is absent, the image
does not support `sev` and the VMM MUST refuse to launch.

## Schema

```cddl
sev = {
  "version"  => uint,                  ; schema version, currently 1
  "dtb"      => tstr,                  ; PE section name; see dtb.md
  "actions"  => [+ sev-action],        ; ordered launch recipe
}

sev-action = sev-load / dtbo
           / sev-id-block / sev-id-auth
           / sev-vmsa / sev-secrets / sev-cpuid

sev-load = {
  "type"        => "load",
  "section"     => tstr,                ; PE section name to load
  ? "measured"  => bool,                ; default true
}
```

VMMs MUST reject sections with an unrecognized `version`, an unknown
top-level key, or an unknown action `type` value.

The `actions` array is processed in order. Each action's `type` selects its
schema and the launch step that consumes it:

| Type           | Step | Consumed by                                              |
| -------------- | ---- | -------------------------------------------------------- |
| `load`         | 4    | `SNP_LAUNCH_UPDATE` (normal page)                        |
| `dtbo`         | 4    | VMM writes overlay into the section; not measured        |
| `sev:vmsa`     | 4    | `SNP_LAUNCH_UPDATE` with `page_type=vmsa`                |
| `sev:secrets`  | 4    | `SNP_LAUNCH_UPDATE` with `page_type=secrets` (PSP fills) |
| `sev:cpuid`    | 4    | `SNP_LAUNCH_UPDATE` with `page_type=cpuid` (VMM fills)   |
| `sev:id-block` | 5    | `SNP_LAUNCH_FINISH` (id_block)                           |
| `sev:id-auth`  | 5    | `SNP_LAUNCH_FINISH` (id_auth)                            |

## Launch model

The `sev` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with the following SEV-SNP behavior layered on:

| Step          | API                                                                 | Inputs                                                                                             |
| ------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| 3. Initialize | `SNP_LAUNCH_START`                                                  | host-supplied launch policy (see [Launch policy](#launch-policy) below)                            |
| 4. Update     | `SNP_LAUNCH_UPDATE` per `load`/`sev:vmsa`/`sev:secrets`/`sev:cpuid` | actions processed in array order; page type determined by action type                              |
| 5. Finalize   | `SNP_LAUNCH_FINISH`                                                 | `sev:id-block` + `sev:id-auth` from the image; `host_data` is deployer-supplied (not in the image) |

## Launch policy

The launch policy passed to `SNP_LAUNCH_START` is **host-supplied** — the
VMM accepts it via VMM-defined input (CLI flag, config file, etc.),
which is out of scope for PMI. The format is the 64-bit POLICY field as
defined in the AMD SEV-SNP firmware ABI.

If `sev:id-block` is present, `sev:id-auth` MUST also be present, and the
host launch policy MUST be compatible with the policy field embedded in
the signed ID block; the VMM MUST verify compatibility and refuse to
launch on mismatch.

If `sev:id-block` is absent, the host has unconstrained latitude over the
launch policy.

The launch policy is not measured; it appears in the attestation report
for remote verification. A remote verifier MUST check policy fields in
the attestation report — the launch digest alone does not establish
policy properties.

## Action definitions

### `load`

`sev` extends the [base `load` action](vm.md#load-action) with one
optional field:

```cddl
sev-load = {
  "type"        => "load",
  "section"     => tstr,
  ? "measured"  => bool,            ; default true
}
```

When `measured` is `true` (the default), the loaded bytes are fed to
`SNP_LAUNCH_UPDATE` and contribute to the launch digest. The distinction
between on-disk data and zero-fill matters: on-disk bytes are measured
as normal data pages; zero-filled bytes are measured via
`SNP_LAUNCH_UPDATE` with `page_type=zero`, which validates pages as
zero without transferring data and produces a different measurement
than loading actual zeros as data pages. VMM implementations MUST NOT
substitute data-page loads for zero-page operations or vice versa.

When `measured` is `false`, the bytes are still loaded into guest
memory but are not fed to the measurement API — used for VMM-supplied
data that the verifier does not need to bind to.

The VMM loads pages from the lowest GPA to the highest within each
section, so measurement is deterministic for a given action ordering.

### `sev:id-block`

The 96-byte SEV ID block passed to `SNP_LAUNCH_FINISH`.

```cddl
sev-id-block = {
  "type"    => "sev:id-block",
  "section" => tstr,                ; PE section containing the 96-byte ID block
}
```

The PE section MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`) and contain
exactly the 96 bytes the AMD SEV-SNP ABI defines for the ID block. The VMM
reads these bytes and passes them to `SNP_LAUNCH_FINISH` as `id_block`.

### `sev:id-auth`

The SEV ID auth info (signatures over the ID block) passed to
`SNP_LAUNCH_FINISH`.

```cddl
sev-id-auth = {
  "type"    => "sev:id-auth",
  "section" => tstr,                ; PE section containing the auth info
}
```

The PE section MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`) and contain
the ID auth info structure (~4 KiB; ECDSA P-384 signatures). The VMM reads
these bytes and passes them to `SNP_LAUNCH_FINISH` as `id_auth`.

### `sev:vmsa`

A 4 KiB page containing the VMPL0 BSP register state at launch.

```cddl
sev-vmsa = {
  "type"    => "sev:vmsa",
  "section" => tstr,                ; PE section containing the 4 KiB VMSA page
}
```

The VMM loads the page via `SNP_LAUNCH_UPDATE` with `page_type=vmsa`. The
page is measured with its actual content.

### `sev:secrets`

A 4 KiB page the PSP populates with platform secrets at launch.

```cddl
sev-secrets = {
  "type"    => "sev:secrets",
  "section" => tstr,                ; PE section reserving the address range
}
```

The referenced PE section MUST be a zero section (`SizeOfRawData == 0`,
`VirtualSize == 4096`). The VMM loads the page via `SNP_LAUNCH_UPDATE` with
`page_type=secrets`; the PSP populates it with secrets at launch. The page is
measured with zero content in the launch digest (the GPA is bound, the
content is not).

### `sev:cpuid`

A 4 KiB page the VMM fills with the CPUID table to expose to the guest.

```cddl
sev-cpuid = {
  "type"    => "sev:cpuid",
  "section" => tstr,                ; PE section reserving the address range
}
```

The referenced PE section MUST be a zero section (`SizeOfRawData == 0`,
`VirtualSize == 4096`). The VMM fills the CPUID table with values it wants to
expose; the PSP validates entries and rejects impossible values. The page is
loaded via `SNP_LAUNCH_UPDATE` with `page_type=cpuid` and measured with zero
content in the launch digest.
