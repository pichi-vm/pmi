# AMD SEV 3.0 Platform Binding

## Platform name

`"sev"` (the key used in the [PMI index](../../index.md)'s `platforms` map).

The convention is to carry the SEV manifest in a PE section named `.pmi.sev`,
but the index is authoritative — any name works.

## PE section naming convention

PE sections specific to SEV 3.0 use the `.sev.` prefix (e.g., `.sev.vms`,
`.sev.sec`, `.sev.cpu`, `.sev.svm`, `.sev.pol`, `.sev.idb`, `.sev.ida`). This
is a convention, not a requirement; segments reference PE sections by name.

## Segment types

The SEV binding defines the following segment types. All segments in a
`.pmi.sev` manifest target SEV — there is no platform filter.

### Launch-input types

These types feed the SEV launch APIs at the corresponding step. They are not
loaded into guest memory.

| Type                 | Step | Backing PE section                                | Consumed by                        |
| -------------------- | ---- | ------------------------------------------------- | ---------------------------------- |
| `"pmi:sev:policy"`   | 4    | Encoded policy fields (see [Policy](#policy))     | `SNP_LAUNCH_START` (policy, gosvw) |
| `"pmi:sev:id-block"` | 8    | 96-byte SEV ID block                              | `SNP_LAUNCH_FINISH` (id_block)     |
| `"pmi:sev:id-auth"`  | 8    | SEV ID auth info (~4 KiB; ECDSA P-384 signatures) | `SNP_LAUNCH_FINISH` (id_auth)      |

If `pmi:sev:id-block` is present, `pmi:sev:id-auth` MUST also be present. The
VMM extracts the policy word from the signed ID block and uses it for
`SNP_LAUNCH_START`; this means a signed launch derives policy from the ID
block rather than from a separate `pmi:sev:policy` segment. A manifest MAY
include both — in which case the VMM MUST verify the `pmi:sev:policy` value
matches the policy embedded in the ID block.

PE sections for launch-input types MUST be non-loaded
(`IMAGE_SCN_MEM_DISCARDABLE`) so that UEFI loaders also skip them. The VMM
reads the bytes directly from the PE file; they are not mapped into guest
memory.

### Page-load types

These types ride `SNP_LAUNCH_UPDATE` during step 6, loading pages into guest
memory at `VirtualAddress`.

| Type                | Step | Backing PE section                         | Consumed by                                             |
| ------------------- | ---- | ------------------------------------------ | ------------------------------------------------------- |
| `"pmi:sev:vmsa"`    | 6    | 4 KiB VMPL0 BSP register state             | `SNP_LAUNCH_UPDATE` with `page_type=vmsa`               |
| `"pmi:sev:secrets"` | 6    | Zero section (`SizeOfRawData == 0`, 4 KiB) | `SNP_LAUNCH_UPDATE` with `page_type=secrets`; PSP fills |
| `"pmi:sev:cpuid"`   | 6    | Zero section (`SizeOfRawData == 0`, 4 KiB) | `SNP_LAUNCH_UPDATE` with `page_type=cpuid`; VMM fills   |

- **`pmi:sev:vmsa`**: image-authored on disk; measured with actual content.
- **`pmi:sev:secrets`**: zero on disk; PSP populates with secrets at launch.
  Measured with zero content in the launch digest (GPA is bound, content is
  not).
- **`pmi:sev:cpuid`**: zero on disk; VMM fills with the CPUID values it wants
  to expose; PSP validates and rejects impossible values. Measured with zero
  content in the launch digest.

Ordinary data segments on SEV (e.g., `.sev.svm`, `.ovmf`, `.linux`) use the
default `"pmi:load"` type.

## Policy

The `pmi:sev:policy` segment's PE section contains a CBOR-encoded map carrying
the policy fields the image requires for `SNP_LAUNCH_START`:

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

### Deployer additions

A deployer MAY supply additional policy fields to the VMM via VMM-defined
input (CLI flag, config file, etc.) — this is out of scope for PMI. The VMM
SHOULD OR the deployer's fields with the image's `pmi:sev:policy` segment,
SHOULD fail the launch if the deployer attempts to weaken an image-required
field, and MUST refuse to launch when a signed `pmi:sev:id-block` is present
and the resulting policy would not match the signed value.

## Execution model mapping

| Step          | API call                                                                      | Manifest input                                                                                        |
| ------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 4. Initialize | `SNP_LAUNCH_START` (policy, gosvw)                                            | `pmi:sev:policy` segment, OR policy fields extracted from `pmi:sev:id-block`                          |
| 5. Pre-load   | (none)                                                                        | —                                                                                                     |
| 6. Segments   | `SNP_LAUNCH_UPDATE` per `pmi:load` and `pmi:sev:{vmsa,secrets,cpuid}` segment | Segments processed in array order; page type determined by segment type                               |
| 7. Post-load  | (none)                                                                        | —                                                                                                     |
| 8. Finalize   | `SNP_LAUNCH_FINISH` (id_block, id_auth, host_data)                            | `pmi:sev:id-block` + `pmi:sev:id-auth` (image); `host_data` is deployer-supplied, not in the manifest |
