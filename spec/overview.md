# Overview

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and
"OPTIONAL" in this document are to be interpreted as described in BCP 14
[[RFC 2119]](https://www.rfc-editor.org/rfc/rfc2119)
[[RFC 8174]](https://www.rfc-editor.org/rfc/rfc8174) when, and only
when, they appear in all capitals, as shown here.

This document defines the goals PMI is shaped to meet and the methods
that meet them. The narrative behind these choices is in
[Motivation](motivation.md); the per-target bindings are in
[`vm`](vm.md), [`sev`](sev.md), [`cca`](cca.md), and [`tdx`](tdx.md);
[Examples](examples.md) walks through concrete images.

## Goals

PMI has four goals:

1. **Security against a malicious hypervisor.** The VMM detects
   incompatibility between the image's declared platform and what the
   host can actually provide, and refuses to launch. The guest's
   validation responsibilities are minimal and well-known — bounded to
   a small, enumerable surface.
2. **Executable format portability.** The same PE bytes load in UEFI
   on bare metal, in any non-CC VMM, and in any CC VMM whose target
   the image declares. UEFI ignores PMI-specific sections; a PMI
   image may simultaneously be a UKI.
3. **Attestation equivalence.** For any two conformant VMMs of the
   same target, given the same PMI image, the image-identity and
   platform-identity fields of the resulting attestation reports are
   bit-identical. Tenant-identity, host-identity, and
   platform-reported fields (firmware/TCB versions, signing keys,
   etc.) may legitimately vary.
4. **Tooling reuse.** Existing PE-based tools work on PMI images
   unchanged. New PMI-specific tools — parsers, DTBO mergers,
   builders, VMMs, in-guest consumers, verifiers, signers,
   inspectors — have narrow contracts and compose across contexts.

### Security against a malicious hypervisor

The primary defense is VMM-side. The VMM reads the image's base
[DTB](dtb.md) and the per-target spec, and validates that the host
can provide every hardware capability declared at the declared GPAs.
A host that cannot satisfy the declaration MUST be refused — the
launch never starts.

The residual defense is guest-side. The host's one channel for
contributing to the platform after launch is the
[`dtbo` overlay](vm.md#dtbo-overlay), restricted by a narrow
content allowlist (four categories). The in-guest PMI consumer
validates the overlay against the allowlist and a small set of
structural rules (FDT well-formedness, allowlisted nodes and
properties, address-bearing values in canonical bounds and
non-overlapping with the base DTB, phandle resolution, bounded
length) and refuses to hand off to the kernel if any check fails.

The narrow allowlist is what keeps the guest's validation surface
tractable: the consumer is a small, enumerable piece of code that any
reviewer can audit end-to-end.

The parties and their trust placement:

| Party             | Supplies                                                              | Trust under `vm`     | Trust under CC targets                       |
| ----------------- | --------------------------------------------------------------------- | -------------------- | -------------------------------------------- |
| Image author      | The PMI image bytes                                                   | Trusted              | Trusted                                      |
| Deployer / tenant | Tenant identity (signatures, hashes that bind to the deployer)        | N/A                  | Trusted for tenant binding only              |
| VMM / host        | Memory allocation, ABI calls into the firmware/module, dtbo content, host identity | Trusted              | Adversarial (outside the trust boundary)     |
| PMI consumer      | In-guest validation, dtbo merge, kernel handoff                       | Trusted by the guest | Trusted by the guest, measured into launch identity |

### Executable format portability

A PMI image is one PE binary. The PE container is universally
understood by PE-based loaders (UEFI, Windows, Wine). PMI's extension
to PE is a set of non-loaded sections whose names begin with
`.pmi.` — one per launch target the image supports. Because these
sections are flagged `IMAGE_SCN_MEM_DISCARDABLE`, PE loaders that do
not know about PMI ignore them. The same image bytes therefore boot:

- on bare metal, where UEFI executes the UKI-style EFI stub
- under a non-CC VMM, which reads `.pmi.vm` and follows its recipe
- under a confidential VMM, which reads `.pmi.sev` / `.pmi.tdx` /
  `.pmi.cca` and follows its recipe

PMI is compatible with UKI, not a flavor of it. An image that
contains only firmware (for OVMF-loads-kernel-from-disk modes), or
only confidential-VM content, is equally valid.

### Attestation equivalence

For any two conformant VMMs of the same target, the image-identity
and platform-identity fields of the attestation report MUST be
bit-identical for the same PMI image. The cryptographic measurement
register (SEV-SNP launch digest, CCA RIM, TDX MRTD) is included
under this rule.

Tenant-identity, host-identity, and platform-reported fields
(firmware/TCB versions, signing keys, etc.) MAY legitimately vary.
Equivalence is therefore tested under a mask that zeroes out the
legitimately-varying fields.

This is the verifier's ergonomic test: given a PMI image and the
relevant vendor specs, a verifier MUST be able to recompute the
expected image+platform identity fields without running the workload,
and compare to what the attestation report shows.

### Tooling reuse

PMI is shaped so existing PE-based tools (`objcopy`, `objdump`,
`sbsign`, `ukify`, `systemd-stub`, UEFI loaders) work on PMI images
unchanged. PMI uses no novel PE features; PE-aware tools that
strip `IMAGE_SCN_MEM_DISCARDABLE` sections by default are the only
known hazard.

New PMI-specific tools are shaped to have narrow contracts and
compose across contexts:

- The target-spec parser is reusable in builders, VMMs, in-guest
  consumers, verifiers, OCI inspectors, and debuggers.
- The DTBO overlay applier is reusable in a small bootloader, in a
  kernel-side pre-handoff stub, and in build-time validation.
- A tenant-identity signer (e.g., for SEV id-block / id-auth) is
  reusable across image authors and tenants.
- Per-target VMM logic composes across hypervisors (KVM, HVF, WHP,
  future ports) because every conformant VMM reads the same wire
  format and applies the same rules.

## Methods

The four goals are delivered through four methods. Each method
serves one or more goals.

### Platform definition inversion → goals (1) and (3)

The image declares the platform it expects to run on. The VMM and any
in-guest consumer comply with the declaration or refuse to launch.
The host has no input into the platform contract.

PMI distinguishes four categories of identity that may appear in an
attestation report:

| Category          | What it is                                                                                                                                | Source                                                          | Appears in measurement? |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | ----------------------- |
| Image identity    | The bytes that constitute the guest workload — kernel, initrd, command line, firmware, any loaded PE section content                     | PMI image                                                       | Yes                     |
| Platform identity | The hardware contract the workload expects — base DTB, boot vCPU register state, TD/realm attributes (ATTRIBUTES, XFAM, RmiRealmParams, launch policy) | PMI image                                                       | Yes                     |
| Tenant identity   | Hash or signature that binds a deployment to a particular tenant — SEV id-block/id-auth, TDX MRCONFIGID/MROWNER/MROWNERCONFIG, CCA RPV     | PMI image (when tenant is the image author) or runtime input    | No — separate channel   |
| Host identity     | Per-deployment values the host supplies — SEV `host_data`, VMM-internal config (max vCPUs, EPTP controls, aux granule addresses, etc.)    | Runtime input                                                   | No — separate channel   |

**Image identity and platform identity MUST be deterministic functions
of the PMI image bytes alone.** Together they produce the launch
measurement.

**Tenant identity** MAY be PMI-bound (when the tenant is the image
author) or runtime-supplied.

**Host identity** is always runtime-supplied — PMI never declares it.

**Tenant identity and host identity MUST NOT contribute to the
cryptographic measurement register.** They are surfaced in the
attestation report through separate firmware channels (e.g.,
`SNP_LAUNCH_FINISH`, the Realm Token, TDREPORT fields outside MRTD).

This inversion delivers goal (1) because the host cannot substitute,
omit, or relocate any platform value without being caught by the
VMM's host-conformance check or by the guest's narrow
consumer-validation rules. It delivers goal (3) because every measured
input is image-determined, leaving no degrees of freedom in which two
conformant VMMs of the same target could diverge.

### PE-as-base → goal (2)

A PMI image is a PE binary. PMI extends PE with non-loaded sections
whose names begin with `.pmi.` — one per launch target the image
supports. PMI imposes alignment rules on PE sections that allow
zero-copy loading with 2 MiB huge pages, and requires target-spec
sections to be flagged `IMAGE_SCN_MEM_DISCARDABLE` so existing PE
loaders ignore them. See [pe.md](pe.md) for the full PE constraints
and page-granularity rules.

A PMI image MAY also be structured as a UKI (carrying `.linux`,
`.initrd`, `.cmdline`, and an EFI stub) so the same bytes boot on
bare metal under UEFI.

### Self-contained byte sections and narrow per-target CBOR → goal (4)

Each target's spec is carried as CBOR in its own PE section
(`.pmi.<target>`) and is self-contained: a tool that handles one
target does not need to read or parse other targets' chapters.

Vendor-defined data structures — AMD SEV-SNP `id_block`/`id_auth`,
Arm RMM `RmiRealmParams`/`RmiRecParams`, Intel TDX `TD_PARAMS` — are
carried as opaque byte sections referenced by name from the target
spec, not marshaled into CBOR. PMI mediates structure (which section
holds which blob) but does not redefine vendor-specific semantics.
This offloads semantic work to vendor-spec-aware tooling that exists
anyway and keeps each PMI-specific tool small.

### Pinned encoding and ordering → goals (3) and (4)

For attestation equivalence and verifier reproducibility, every
producer and consumer must agree byte-for-byte on the wire format and
on the order in which measured inputs are submitted to firmware:

- All target specs are encoded as CBOR per
  [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949). Producers MUST
  emit Core Deterministic Encoding (RFC 8949 §4.2.1). Consumers MUST
  reject malformed CBOR and MUST reject duplicate map keys.
- Actions within a target's `actions` array are processed in array
  order. Within each action's PE section, the VMM submits pages from
  the lowest GPA to the highest. These two rules together pin the
  order in which measured inputs reach firmware.
- Each per-target chapter cites the vendor specification and version
  it depends on (AMD SEV-SNP firmware ABI publication and revision;
  Arm RMM specification DEN0137 revision; Intel TDX Module
  specification revision).
- Each per-target chapter SHOULD include normative reference
  vectors — at minimum a positive vector (a PMI image whose decoded
  spec matches an explicit byte sequence) and a negative vector (an
  image that MUST be rejected, with the rejecting rule cited).

## Targets

PMI defines one **target** per launch path the image supports. A
target is a self-contained CBOR spec carried in its own PE section. A
VMM targeting one of them reads that target's section, ignores the
others, and executes the recipe it finds there.

The currently defined targets are:

| Target          | PE section | Notes                                       |
| --------------- | ---------- | ------------------------------------------- |
| [`vm`](vm.md)   | `.pmi.vm`  | Non-CC virtual machines                     |
| [`sev`](sev.md) | `.pmi.sev` | AMD SEV 3.0 (SEV-SNP) confidential VMs      |
| [`tdx`](tdx.md) | `.pmi.tdx` | Intel TDX confidential VMs (working draft)  |
| [`cca`](cca.md) | `.pmi.cca` | Arm CCA confidential VMs (working draft)    |

[`vm`](vm.md) defines the [base launch model](vm.md#launch-model);
CC targets inherit it and describe only their cryptographic deltas.
A VMM targeting one of them reads the corresponding `.pmi.<target>`
section — there is no fallback or selection logic beyond that.
