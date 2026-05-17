# AMD SEV 3.0 Platform Binding

## Platform key

`"sev"`

## Segment annotations

PE sections specific to SEV 3.0 use the `.sev.` prefix (e.g., `.sev.vms`,
`.sev.sec`, `.sev.cpu`, `.sev.svm`).

Segment-level platform annotations for SEV 3.0 use string values in the
segment's `"platforms"` map:

| Annotation  | Behavior                                              |
| ----------- | ----------------------------------------------------- |
| `null`      | Load as normal measured data                          |
| `"vmsa"`    | Load via `SNP_LAUNCH_UPDATE` with `page_type=vmsa`    |
| `"secrets"` | Load via `SNP_LAUNCH_UPDATE` with `page_type=secrets` |
| `"cpuid"`   | Load via `SNP_LAUNCH_UPDATE` with `page_type=cpuid`   |

- **vmsa**: The PE section contains the 4K VMPL0 BSP register state. This is
  image-authored data on disk, measured with its actual content.
- **secrets**: The PE section is a zero section (`SizeOfRawData == 0`). The
  VMM allocates the page and the PSP populates it with secrets at launch.
  Measured with zero content in the launch digest (GPA is bound, content is
  not).
- **cpuid**: The PE section is a zero section (`SizeOfRawData == 0`). The VMM
  fills the CPUID table with the values it wants to expose to the guest. The
  PSP validates the entries and rejects impossible values. Measured with zero
  content in the launch digest (GPA is bound, content is not).

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

| Step          | API call                                                             |
| ------------- | -------------------------------------------------------------------- |
| 4. Initialize | `SNP_LAUNCH_START` (merged policy)                                   |
| 5. Pre-load   | (none)                                                               |
| 6. Segments   | `SNP_LAUNCH_UPDATE` per segment (page type determined by annotation) |
| 7. Post-load  | (none)                                                               |
| 8. Finalize   | `SNP_LAUNCH_FINISH` (id_block)                                       |
