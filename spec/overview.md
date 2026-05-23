# Overview

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and
"OPTIONAL" in this document are to be interpreted as described in BCP 14
[[RFC 2119]](https://www.rfc-editor.org/rfc/rfc2119)
[[RFC 8174]](https://www.rfc-editor.org/rfc/rfc8174) when, and only
when, they appear in all capitals, as shown here.

This document defines the goals PMI is shaped to meet and the
methods that meet them. The narrative behind these choices is in
[Motivation](motivation.md); the action mechanism is on the
[Actions](actions.md) page; the extensibility contract is on the
[Extensions](extensions.md) page; the per-target bindings are in
[`vm`](vm.md), [`sev`](sev.md), [`cca`](cca.md), and [`tdx`](tdx.md);
[Examples](examples.md) walks through concrete images.

PMI is intentionally narrow. It is a substrate: a PE-based container
format, per-target CBOR launch recipes, and an action mechanism that
drives the firmware ABIs each target exposes. Platform identity,
attestation policy, host-conformance checking, and the
measured-vs-unmeasured boundary as a security argument belong to
**upper layers** (e.g., dillo) that build on top of PMI via the
[Extensions](#extensions) namespace.

## Goals

PMI has three goals, one per [problem](motivation.md):

1. **Executable format portability.** The same PE bytes load in UEFI
   on bare metal, in any non-CC VMM, and in any CC VMM whose target
   the image declares. UEFI ignores PMI-specific sections; a PMI
   image may simultaneously be a UKI.
2. **Uniform approach across targets.** PMI uses the same shape for
   every target — same CBOR structure, same action types, same
   byte-section pattern for vendor blobs, same encoding rules. Native
   vendor semantics differ per target; PMI's interface on top of them
   does not.
3. **Tooling reuse.** Existing PE-based tools work on PMI images
   unchanged. New PMI-specific tools — parsers, builders, VMMs,
   in-guest consumers, verifiers, signers, inspectors — have narrow
   contracts and compose across contexts.

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
byte-section pattern for vendor-defined blobs, and the same
encoding and ordering rules. The per-target chapters differ only in
their target-specific deltas: what kinds of `load` and `fill`
mean, which vendor structures get carried as byte sections, what
the underlying firmware does with each action.

This means an image author learns one format and ships one image to
N targets; a consumer parses one CBOR shape regardless of which
target it's running under; tools (parser, signer, inspector) work
uniformly across targets.

Native vendor differences (SEV-SNP's launch digest vs. CCA's RIM
vs. TDX's MRTD, AMD's id-block vs. CCA's RPV vs. TDX's MRCONFIGID,
the firmware ABIs each target rides) are preserved — PMI is a
framework on top of native target semantics, not a replacement for
them. Everything PMI itself defines applies uniformly across every
target.

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
- A tenant-identity signer (e.g., for SEV id-block / id-auth) is
  reusable across image authors and tenants.
- Per-target VMM logic composes across hypervisors (KVM, HVF, WHP,
  future ports) because every conformant VMM reads the same wire
  format and applies the same rules.

## Methods

The three goals are delivered through three methods. Each method is
target-agnostic by design and serves multiple goals.

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

### Self-contained byte sections and narrow per-target CBOR → goals (2) and (3)

Each target's spec is carried as CBOR in its own PE section
(`.pmi.<target>`) and is self-contained: a tool that handles one
target does not need to read or parse other targets' chapters.

Vendor-defined data structures — AMD SEV-SNP `id_block`/`id_auth`,
Arm RMM and Intel TDX vendor blobs as future targets adopt them —
are carried as opaque byte sections referenced by name from the
target spec, not marshaled into CBOR. PMI mediates structure (which
section holds which blob) but does not redefine vendor-specific
semantics. This offloads semantic work to vendor-spec-aware tooling
that exists anyway and keeps each PMI-specific tool small.

The same byte-section pattern applies to every target: an image
author who learns the convention once can use it for every
vendor-defined blob across `sev`, `cca`, and `tdx`. PMI tooling
treats vendor blobs uniformly — read the section by name, pass
through the bytes — regardless of which target's spec referenced
them.

### Pinned encoding and ordering → goals (2) and (3)

For tooling determinism and for any verifier that needs to reproduce
what PMI submits to firmware, every producer and consumer must agree
byte-for-byte on the wire format and on the order in which inputs
reach firmware:

- All target specs are encoded as CBOR per
  [RFC 8949](https://www.rfc-editor.org/rfc/rfc8949). Producers MUST
  emit Core Deterministic Encoding (RFC 8949 §4.2.1). Consumers MUST
  reject malformed CBOR and MUST reject duplicate map keys.
- Actions within a target's `actions` array are processed in array
  order. Within each action's PE section, the VMM submits pages from
  the lowest GPA to the highest. These two rules together pin the
  order in which inputs reach firmware.
- Each per-target chapter cites the vendor specification and version
  it depends on (AMD SEV-SNP firmware ABI publication and revision;
  Arm RMM specification DEN0137 revision; Intel TDX Module
  specification revision).

The encoding and ordering rules apply uniformly across every target;
a producer learns one set of rules; any layered spec that wants to
make stronger claims (attestation equivalence, measurement
stability) builds them on top of PMI's determinism guarantees.

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

Each target is itself a [registered extension](extensions.md#extension-registry):
it owns its name in the registry and the corresponding
`.pmi.<target>` PE section. New targets are added through the
[target extension point](extensions.md#4-new-targets-registered-only)
and require registration.

[`vm`](vm.md) defines the [base launch model](vm.md#launch-model);
CC targets inherit it and describe only their firmware-specific
deltas. A VMM targeting one of them reads the corresponding
`.pmi.<target>` section — there is no fallback or selection logic
beyond that.

## Extensions

PMI is a substrate for upper layers — hypervisors, in-guest stubs,
image schemas — that want to carry layer-specific data alongside
PMI's firmware-bound launch recipe. The extensibility contract is
a single namespacing convention: unprefixed names (`version`,
`actions`, `secrets`, etc.) are reserved for PMI; names of the
form `<layer>:<name>` (e.g., `dillo:dtb`) belong to the named
upper layer; loaders MUST reject any name they do not understand.

See [Extensions](extensions.md) for the common target shape,
the three extension points (target attributes, new action types,
per-action kind customization), and a worked example.
