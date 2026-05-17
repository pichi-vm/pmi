# AMD SEV 3.0 Platform Binding

## Platform key

`"sev"`

## PE section naming convention

PE sections specific to SEV 3.0 use the `.sev.` prefix (e.g., `.sev.vms`,
`.sev.sec`, `.sev.cpu`, `.sev.svm`).

## Segment types

SEV 3.0 defines the following segment `type` values. Each MUST be paired with a
`platforms` filter that includes `"sev"`; the VMM rejects an SEV-typed segment
that could be selected on a non-SEV launch.

| Type                | Behavior                                                                                                 |
| ------------------- | -------------------------------------------------------------------------------------------------------- |
| `"pmi:sev:vmsa"`    | Load via `SNP_LAUNCH_UPDATE` with `page_type=vmsa`. PE section contains the 4K VMPL0 BSP register state. |
| `"pmi:sev:secrets"` | Reserve a 4K zero section; the PSP populates it with secrets at launch.                                  |
| `"pmi:sev:cpuid"`   | Reserve a 4K zero section; the VMM fills the CPUID table, the PSP validates entries.                     |

- **`pmi:sev:vmsa`** carries image-authored data on disk. Measured via
  `SNP_LAUNCH_UPDATE` with its actual content.
- **`pmi:sev:secrets`** references a zero PE section (`SizeOfRawData == 0`,
  `VirtualSize > 0`). Measured with zero content in the launch digest (GPA is
  bound, content is not).
- **`pmi:sev:cpuid`** references a zero PE section. The VMM fills the CPUID
  table with values it wants to expose to the guest; the PSP validates entries
  and rejects impossible values. Measured with zero content in the launch digest
  (GPA is bound, content is not).

Ordinary data segments on SEV (e.g., `.sev.svm`, `.ovmf`, `.linux`) use the
default `"pmi:load"` type with a `platforms` filter naming `"sev"` where the
section is SEV-only.

## Policy schema

```cddl
; See SEV Secure Nested Paging Firmware ABI §4.3, Table 9.
sev-policy = {
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
}

sev-abi = {
  ? "major" => uint,                     ; [15:8] minimum ABI major version
  ? "minor" => uint,                     ; [7:0] minimum ABI minor version
}
```

Policy is not measured. It is passed to `SNP_LAUNCH_START` and appears in the
attestation report for remote verification. See [policy.md](../policy.md) for
merge semantics.

## Execution model mapping

| Step          | API call                                                       |
| ------------- | -------------------------------------------------------------- |
| 4. Initialize | `SNP_LAUNCH_START` (merged policy)                             |
| 5. Pre-load   | (none)                                                         |
| 6. Segments   | `SNP_LAUNCH_UPDATE` per segment (page type determined by type) |
| 7. Post-load  | (none)                                                         |
| 8. Finalize   | `SNP_LAUNCH_FINISH` (id_block)                                 |
