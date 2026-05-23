# Overview

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and
"OPTIONAL" in this document are to be interpreted as described in BCP 14
[[RFC 2119]](https://www.rfc-editor.org/rfc/rfc2119)
[[RFC 8174]](https://www.rfc-editor.org/rfc/rfc8174) when, and only
when, they appear in all capitals, as shown here.

This document defines the categories of data PMI distinguishes, the
goals PMI is shaped to meet, and the methods that meet them. The
narrative behind these choices is in [Motivation](motivation.md);
[Categories](categories.md) defines each category in depth with the
topological mapping that produced them; the per-target bindings are
in [`vm`](vm.md), [`sev`](sev.md), [`cca`](cca.md), and
[`tdx`](tdx.md); [Examples](examples.md) walks through concrete
images.

## Categories

PMI distinguishes six categories of data in any launch. The goals,
methods, and per-target bindings that follow are stated in terms of
these categories.

| Category               | What it is                                                                                                                       | Source                                                       | Measured? | In attestation report?      |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | --------- | --------------------------- |
| **Image identity**     | The workload bytes — kernel, initrd, command line, firmware, any loaded PE section content                                       | PMI image                                                    | Yes       | Yes (in measurement)        |
| **Platform identity**  | The hardware *shape* the workload expects — devices, MMIO, IRQ controller, PCIe (base DTB); boot vCPU state; CC-feature requirements | PMI image                                                    | Yes       | Yes (in measurement)        |
| **Tenant identity**    | A hash or signature binding a deployment to a tenant — SEV id-block/id-auth, TDX MR\*, CCA RPV                                   | PMI image (when tenant is the image author) or runtime input | No        | Yes (separate report field) |
| **Host identity**      | Host-supplied attestation data naming a host operator — e.g., SEV `HOST_DATA`                                                    | Runtime input                                                | No        | Yes (separate report field) |
| **Deployer policy**    | Operational metadata the verifier checks against policy — SEV-SNP POLICY when no ID block is present                             | Runtime input                                                | No        | Yes (separate report field) |
| **Instance accidents** | Per-launch sizing and wiring with no identity meaning — vCPU count, memory size, NUMA (dtbo); aux granules; EPTP; allocator output | Runtime input                                                | No        | No                          |

Image identity and platform identity contribute to the cryptographic
measurement (SEV-SNP launch digest, CCA RIM, TDX MRTD). Tenant
identity, host identity, and deployer policy reach the attestation
report through separate firmware channels (`SNP_LAUNCH_FINISH`, the
Realm Token, TDREPORT fields outside MRTD). Instance accidents
appear in no attestation field at all.

See [Categories](categories.md) for each category in depth, the
treatment of launch policy as a non-category, and a decision
procedure for classifying new target parameters. Per-target
enumerations live in each target's chapter.

## Goals

PMI has six goals:

1. **Executable format portability.** The same PE bytes load in UEFI
   on bare metal, in any non-CC VMM, and in any CC VMM whose target
   the image declares. UEFI ignores PMI-specific sections; a PMI
   image may simultaneously be a UKI.
2. **Uniform approach across targets.** PMI uses the same shape for
   every target — same CBOR structure, same categories framework,
   same action types, same base-DTB-vs-dtbo split, same byte-section
   pattern for vendor blobs, same encoding rules. Native vendor
   semantics differ per target; PMI's interface on top of them does
   not.
3. **Security against a malicious hypervisor.** The image declares
   image and platform identity; under CC, a host that substitutes any
   declared value produces a launch measurement that does not match
   the expected value, and a remote verifier rejects the launch. The
   guest's residual validation responsibilities are minimal and
   well-known.
4. **Measurement stability.** Scaling the same image to a different
   deployment size produces the same launch measurement as the
   original. Image identity and platform identity are measured;
   instance accidents (resource allocation, allocator output,
   VMM-internal configuration) are not.
5. **Attestation equivalence.** For any two conformant VMMs of the
   same target, given the same PMI image, the image-identity and
   platform-identity fields of the resulting attestation reports are
   bit-identical. Tenant-identity, host-identity, and
   platform-reported fields (firmware/TCB versions, signing keys,
   etc.) may legitimately vary.
6. **Tooling reuse.** Existing PE-based tools work on PMI images
   unchanged. New PMI-specific tools — parsers, DTBO mergers,
   builders, VMMs, in-guest consumers, verifiers, signers,
   inspectors — have narrow contracts and compose across contexts.

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

### Uniform approach across targets

PMI uses the same shape for every target. An image author who
supports `vm`, `sev`, `cca`, and `tdx` writes the same CBOR
structure with the same action types (`load`, `fill`), the same
[categories](#categories) framework, the same base-DTB-vs-dtbo
split, the same byte-section pattern for vendor-defined blobs, and
the same encoding and ordering rules. The per-target chapters
differ only in their target-specific deltas: what kinds of `load`
mean, which vendor structures get carried as byte sections, what
the underlying firmware does with each action.

This means an image author learns one format and ships one image to
N targets; a verifier learns one input structure (the native crypto
differs but PMI's contribution to it is uniform); a consumer parses
one CBOR spec shape regardless of which target it's running under;
tools (parser, DTBO applier, signer, inspector) work uniformly
across targets.

Native vendor differences (SEV-SNP's launch digest vs. CCA's RIM
vs. TDX's MRTD, AMD's id-block vs. CCA's RPV vs. TDX's MRCONFIGID,
the firmware ABIs each target rides) are preserved — PMI is a
framework on top of native target semantics, not a replacement for
them. But everything PMI itself defines applies uniformly across
every target.

### Security against a malicious hypervisor

Under CC targets, the VMM is outside the guest's trust boundary, so
a defense based on the VMM checking itself proves nothing — a
malicious VMM will claim compliance and substitute whatever it
likes. The actual defense is cryptographic. The image declares the
platform identity it expects; the declaration is bound into the
launch measurement at firmware-controlled steps the VMM cannot
bypass; a remote verifier rejects any launch whose measurement
diverges from the expected value. A VMM that substituted any
image-identity or platform-identity value produces a different
measurement, and the verifier catches it before any secret is
released. The VMM's role in "complying" with the image's declaration
is therefore operational — the VMM either produces a launch state
matching the declaration or fails to produce a measurement the
verifier will accept.

The [`dtbo` overlay](vm.md#dtbo-overlay) carries instance accidents
(resource allocation: vCPU count, memory layout, NUMA topology). It
is the one host-controlled input that reaches the guest after the
launch measurement has been finalized. Because it is not measured,
the in-guest PMI consumer (itself inside the trust boundary and
bound into the launch measurement) validates it before merging with
the base DTB. The overlay is restricted by a narrow content
allowlist plus a small set of structural rules (FDT well-formedness,
allowlisted nodes and properties, address-bearing values in
canonical bounds and non-overlapping with the base DTB, phandle
resolution, bounded length). The allowlist is narrow on purpose:
the validator is a small, enumerable piece of code that any reviewer
can audit end-to-end.

Under non-CC `vm` the VMM is inside the guest's trust boundary;
its host-conformance check (refusing to launch when it cannot
provide the declared platform) is the defense, and attestation
does not apply.

The parties and their trust placement:

| Party             | Supplies                                                              | Trust under `vm`     | Trust under CC targets                       |
| ----------------- | --------------------------------------------------------------------- | -------------------- | -------------------------------------------- |
| Image author      | The PMI image bytes (image + platform identity)                       | Trusted              | Trusted                                      |
| Deployer / tenant | Tenant identity (signatures, hashes that bind to the deployer)        | N/A                  | Trusted for tenant binding only              |
| VMM / host        | Memory allocation, ABI calls into the firmware/module, dtbo content (instance accidents), host identity | Trusted              | Adversarial (outside the trust boundary)     |
| PMI consumer      | In-guest validation, dtbo merge, kernel handoff                       | Trusted by the guest | Trusted by the guest, measured into launch identity |

### Measurement stability

A deployer scaling the same image to a different size produces the
same launch measurement as the original. Doubling memory, changing
the vCPU count, or rearranging NUMA topology changes the dtbo
content but does not perturb the measurement; only the
image-identity and platform-identity bytes contribute to it.

This isolation is structural. The base DTB carries platform
identity (image-bound, baked into the image bytes the measurement
covers); the dtbo carries instance accidents (host-supplied at
launch, allowlist-bounded, not measured). Image identity and
platform identity move with the image; instance accidents move with
the deployment; the measurement reflects only the former.

Without this separation, the verifier would need to know the
deployment-time resource sizing in order to recompute the expected
measurement — defeating the point of binding attestation to image
identity. With it, a verifier policy that says "release the secret
when the launch measurement equals X" works across every
deployment of image X, regardless of how the host sized it.

### Attestation equivalence

For any two conformant VMMs of the same target, the image-identity
and platform-identity fields of the attestation report MUST be
bit-identical for the same PMI image. The cryptographic measurement
register (SEV-SNP launch digest, CCA RIM, TDX MRTD) is included
under this rule.

Tenant-identity and host-identity fields MAY legitimately vary
(they're per-deployer or per-host); platform-reported fields
(firmware/TCB versions, signing keys, etc.) MAY also vary
(they're not in PMI's control). Instance accidents do not appear in
the attestation at all. Equivalence is therefore tested under a mask
that zeroes out the legitimately-varying and non-attestation fields.

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

The six goals are delivered through four methods. Each method serves
one or more goals. Goal (2) — uniformity across targets — is served
by every method, because each method is target-agnostic by design.

### PE-as-base → goals (1) and (2)

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

The PE container itself is target-agnostic: the same image carries
sections for every target the image author chooses to support, and
loaders that don't know about a target simply ignore its sections.

### Platform definition inversion → goals (2), (3), (4), and (5)

The image declares image and platform identity; the host complies
or fails to produce a measurement a verifier will accept. The five
[categories](#categories) make this concrete: image identity and
platform identity are PMI-bound; tenant identity MAY be (when the
tenant is the image author); host identity is runtime-supplied;
instance accidents are runtime-supplied and never enter the
measurement.

**Image identity and platform identity MUST be deterministic
functions of the PMI image bytes alone.** Together they produce the
launch measurement.

**Tenant identity** MAY be PMI-bound (when the tenant is the image
author) or runtime-supplied.

**Host identity** is always runtime-supplied — PMI never declares it.

**Tenant identity, host identity, and instance accidents MUST NOT
contribute to the cryptographic measurement register.** Tenant and
host identity are surfaced in the attestation report through
separate firmware channels; instance accidents are not surfaced in
the attestation at all.

This inversion delivers goal (3) because under CC, a host that
substitutes any image- or platform-identity value produces a launch
measurement that does not match the expected value and the verifier
rejects the launch — no secret is released. The dtbo (the residual
host-controlled input carrying instance accidents) is constrained at
the guest by the narrow consumer-validation rules. Under non-CC
`vm` the same declaration drives the VMM's operational
host-conformance check, which fails the launch if the host cannot
provide what the image expects.

It delivers goal (4) because the structural split between platform
identity (image-bound, in the base DTB and the per-target spec) and
instance accidents (host-supplied, in the dtbo) means deployment-time
resource sizing never reaches the measurement. The 4-vCPU and 8-vCPU
deployments of the same image produce the same measurement.

It delivers goal (5) because every measured input is
image-determined, leaving no degrees of freedom in which two
conformant VMMs of the same target could diverge.

It delivers goal (2) because the categories framework is the same
across every target: `vm`, `sev`, `cca`, and `tdx` all use the same
image/platform/tenant/host/accidents partition, and the same
base-DTB-vs-dtbo split. An image author who learns the inversion for
one target knows it for all of them.

### Self-contained byte sections and narrow per-target CBOR → goals (2) and (6)

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

The same byte-section pattern applies to every target: an image
author who learns the convention once can use it for every
vendor-defined blob across `sev`, `cca`, and `tdx`. PMI tooling
treats vendor blobs uniformly — read the section by name, pass
through the bytes — regardless of which target's spec referenced
them.

### Pinned encoding and ordering → goals (2), (5), and (6)

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

The encoding and ordering rules apply uniformly across every
target; a producer learns one set of rules, a verifier implements
one reproduction strategy.

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
