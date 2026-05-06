# Policy

The manifest's `policy` map carries platform launch policy. Each key is a
platform name; each value is a platform-defined policy structure. See the
[manifest schema](README.md#schema) for the top-level structure.

```cddl
policy = {
  ? "sev"     => sev-policy             ; see platforms/sev.md
  ? "tdx"     => tdx-policy             ; see platforms/tdx.md
  ? "cca"     => cca-policy             ; see platforms/cca.md
  ? "native"  => native-policy          ; see platforms/native.md
  * tstr => any,                        ; extension platform types
}
```

Well-known platform keys are defined in each platform's binding specification.
Extensions use `"namespace:type"` (e.g., `"myvendor:custom-tee"`).

## Merge Semantics

Policy is not measured. It is passed to the platform's initialization API (e.g.,
`SNP_LAUNCH_START`) and appears in the attestation report for remote
verification. A remote verifier MUST check the policy fields in the attestation
report — the launch digest alone does not establish policy properties.

The image may embed required policy. A deployer may supply an external policy
that is merged with the image policy before launch. If the merge detects a
conflict (both sides set the same field to different values), the VMM SHOULD
fail the launch with a clear indication of which fields conflict. If the VMM
chooses to launch despite a conflict, it MUST prefer the image's value — no
image should ever launch without its required configuration.

The VMM performs policy merging in
[step 2](../overview.md#vmm-execution-model) of the execution model.

## Merge Algorithm

The VMM merges a deployer-supplied policy into the image policy using a
recursive deep merge.

Given two CBOR values `image` and `deployer`, the function
`merge(image, deployer)` is defined as:

1. If `image` is absent or null, return `deployer`.
2. If `deployer` is absent or null, return `image`.
3. If both `image` and `deployer` are maps, return a new map containing:
   - For each key present in both maps: the key mapped to
     `merge(image[key], deployer[key])`.
   - For each key present only in `image`: the key mapped to `image[key]`.
   - For each key present only in `deployer`: the key mapped to `deployer[key]`.
4. Otherwise (at least one is a non-map value), return `image`.

Rule 4 is the conflict rule: when the image and deployer disagree on a scalar
or when one provides a map and the other a scalar for the same key, the image
value is used. The VMM SHOULD report this conflict to the deployer rather than
silently proceeding. Rule 3 ensures that maps are merged recursively so that
the deployer can fill in fields at any depth without losing image-defined fields
at the same depth.

## Example

```cbor-diag
; Image policy (embedded in .pmi section)
{"sev": {"debug": false, "abi": {"major": 1}}}

; Deployer policy (supplied externally)
{"sev": {"smt": true, "abi": {"minor": 2}}}

; Merged result (step 2 of execution model)
{"sev": {"debug": false, "smt": true, "abi": {"major": 1, "minor": 2}}}
```

The image locked `debug` to `false` and required ABI major version 1. The
deployer enabled SMT and added an ABI minor version constraint. Both are
preserved in the merged result. Had the deployer also set `"debug": true`, the
VMM SHOULD fail the launch and report the conflict on `debug`. If the VMM
proceeds anyway, it MUST use `false` (the image's value).
