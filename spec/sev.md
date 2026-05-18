# `sev` Target

The `sev` target is the AMD SEV 3.0 (SEV-SNP) launch path. The VMM reads the
image's base [DTB](dtb.md), processes the actions list to drive the SEV
launch APIs (`SNP_LAUNCH_START`, `SNP_LAUNCH_UPDATE`, `SNP_LAUNCH_FINISH`),
and starts the guest under SEV protection.

The `sev` target is independent of [`vm`](vm.md). It reuses the
[`load`](load.md) and [`dtbo`](dtbo.md) action type names — with semantics
specified in this document, not inherited from `vm` — and adds its own
launch-specific action types. It does not use `vcpu` (the SEV equivalent
is `sev:vmsa`).

## PE section

A VMM targeting `sev` reads the `.pmi.sev` PE section. The section MUST be
non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is absent, the image
does not support `sev` and the VMM MUST refuse to launch.

## Schema

```cddl
sev = {
  "version"  => uint,                  ; schema version, currently 1
  ? "dtb"    => tstr,                  ; PE section name; see dtb.md
  "actions"  => [+ sev-action],        ; ordered launch recipe
  * tstr => any,                       ; unknown keys ignored
}

sev-action = load / dtbo
           / sev-policy / sev-id-block / sev-id-auth
           / sev-vmsa / sev-secrets / sev-cpuid
```

VMMs MUST reject sections with an unrecognized `version`.

The `actions` array is processed in order. Each action's `type` selects its
schema and the launch step that consumes it:

| Type           | Step | Consumed by                                              |
| -------------- | ---- | -------------------------------------------------------- |
| `load`         | 6    | `SNP_LAUNCH_UPDATE` (normal page)                        |
| `dtbo`         | 6    | VMM writes overlay into the section; not measured        |
| `sev:policy`   | 4    | `SNP_LAUNCH_START` (policy, gosvw)                       |
| `sev:vmsa`     | 6    | `SNP_LAUNCH_UPDATE` with `page_type=vmsa`                |
| `sev:secrets`  | 6    | `SNP_LAUNCH_UPDATE` with `page_type=secrets` (PSP fills) |
| `sev:cpuid`    | 6    | `SNP_LAUNCH_UPDATE` with `page_type=cpuid` (VMM fills)   |
| `sev:id-block` | 8    | `SNP_LAUNCH_FINISH` (id_block)                           |
| `sev:id-auth`  | 8    | `SNP_LAUNCH_FINISH` (id_auth)                            |

Consumers MUST ignore unknown keys but MUST reject unknown action `type`
values.

## Execution model

| Step          | API                                                                 | Inputs                                                                                             |
| ------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| 4. Initialize | `SNP_LAUNCH_START`                                                  | `sev:policy` action, OR policy fields extracted from `sev:id-block`                                |
| 6. Update     | `SNP_LAUNCH_UPDATE` per `load`/`sev:vmsa`/`sev:secrets`/`sev:cpuid` | actions processed in array order; page type determined by action type                              |
| 8. Finalize   | `SNP_LAUNCH_FINISH`                                                 | `sev:id-block` + `sev:id-auth` from the image; `host_data` is deployer-supplied (not in the image) |

If `sev:id-block` is present, `sev:id-auth` MUST also be present. The VMM
extracts the policy word from the signed ID block and uses it for
`SNP_LAUNCH_START`; a signed launch derives policy from the ID block rather
than from a separate `sev:policy` action. The image MAY include both — in
which case the VMM MUST verify the `sev:policy` value matches the policy
embedded in the ID block.

## Action definitions

### `sev:policy`

Carries the policy fields the image requires for `SNP_LAUNCH_START`.

```cddl
sev-policy = {
  "type"    => "sev:policy",
  "section" => tstr,                ; PE section containing the policy CBOR map
}
```

The PE section MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). Its bytes
are a CBOR-encoded map:

```cddl
; See SEV Secure Nested Paging Firmware ABI §4.3, Table 9.
sev-policy-fields = {
  ? "abi"                    => sev-abi, ; [15:0] minimum ABI version
  ? "debug"                  => bool,    ; [19] allow debugging
  ? "migrate-ma"             => bool,    ; [18] allow migration agent association
  ? "single-socket"          => bool,    ; [20] restrict to single socket
  ? "cxl-allow"              => bool,    ; [21] allow CXL devices/memory
  ? "mem-aes-256-xts"        => bool,    ; [22] require AES-256-XTS memory encryption
  ? "rapl-dis"               => bool,    ; [23] require RAPL disabled
  ? "ciphertext-hiding-dram" => bool,    ; [24] require ciphertext hiding for DRAM
  ? "page-swap-disable"      => bool,    ; [25] disable SNP_PAGE_MOVE/SWAP_OUT/SWAP_IN
  ? "smt"                    => bool,    ; [16] allow SMT
  ? "gosvw"                  => bstr,    ; 16-byte Guest OS Visible Workarounds
}

sev-abi = {
  ? "major" => uint,                     ; [15:8] minimum ABI major version
  ? "minor" => uint,                     ; [7:0] minimum ABI minor version
}
```

The VMM encodes the named fields into the SEV policy word and passes it
(along with `gosvw`) to `SNP_LAUNCH_START`. Missing fields default to the
SEV-defined defaults.

Policy is not measured; it appears in the attestation report for remote
verification. A remote verifier MUST check policy fields in the attestation
report — the launch digest alone does not establish policy properties.

A deployer MAY supply additional policy fields to the VMM via VMM-defined
input (CLI flag, config file, etc.) — this is out of scope for PMI. The VMM
SHOULD OR the deployer's fields with the image's `sev:policy` action, SHOULD
fail the launch if the deployer attempts to weaken an image-required field,
and MUST refuse to launch when a signed `sev:id-block` is present and the
resulting policy would not match the signed value.

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
