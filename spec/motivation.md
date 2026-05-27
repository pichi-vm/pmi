# Motivation

PMI exists to achieve three goals:

1. portability across targets (bare metal, VM, AMD SEV, Arm CCA and Intel TDX).
2. portability of safe platform definition and attestation
3. reuse of existing tooling and formats

PMI is intentionally narrow. It covers how to load pages into a VM and how to
safely communicate the VM platform definition without sacrificing portability
across targets, attestation measurements or VM implementations. Higher-level
concerns are out of scope for PMI.

## 1. Portability Across Targets

A single Linux workload — the same kernel, initrd, and command line —
increasingly has to run in many shapes: bare metal under UEFI, a direct-boot VM,
a VM under guest firmware (OVMF), or a confidential VM behind a service module
(COCONUT-SVSM, a paravisor). Existing image formats each assume one shape.
PE/UKI assumes UEFI runs an EFI stub; IGVM assumes a paravisor-style
confidential boot. An author who needs more than one shape ships more than one
artifact, with parallel build, signing, and distribution for each.

This is wasteful because the unit of distribution is one artifact: pulled from a
registry, cached, signed once, scanned once, attested once, addressed by one
content hash. Splitting a workload across per-shape artifacts duplicates that
whole pipeline.

![Boot pipelines: bare metal versus modern VM](images/boot-modes.excalidraw.svg)

PMI is one image that boots every shape — bare metal under UEFI, a standard VM,
or a confidential VM — from the same bytes.

## 2. Portable, Safe Platform Definition and Attestation

Confidential Computing imposes two competing requirements on a VM:

- **The platform definition must be safe.** The host describes the guest's
  platform — memory map, MMIO and IO regions, CPU topology — and a malicious
  host can attack the guest through that description.

- **Attestation must not be fragile.** The guest's measurement must be portable
  and deterministic: the same image should attest the same way on every VMM,
  regardless of how the host provisioned it.

The obvious way to make the platform definition safe is to measure it, so the
guest and a verifier can detect tampering. But measuring it folds the host's
resource decisions into the guest's identity: the same image then attests
differently depending on how much memory it was given or which hypervisor ran
it, and even minor VMM version bumps perturb the measurement. That makes
attestation fragile. Satisfying one requirement this way sacrifices the other —
and PMI's position is that you should not have to choose.

### Why Safety Pressure Pushes Toward Measurement

The platform description gives the VMM significant flexibility to attack the
guest — for example, by defining overlapping device memory regions to bypass
access controls and exfiltrate data. The most dangerous case is executable code.
On `x86`, platform firmware communicates the layout via ACPI, which defines a
bytecode language (AML) that runs arbitrary code in the most privileged part of
the OS. Under the old threat model — firmware trusted — this was fine; under
Confidential Computing it is a direct path for the host to attack the guest.
This is not theoretical: AMD published [AMD-SB-3012][amdsb] demonstrating a
practical attack, and the recommended mitigation is to **measure** the ACPI
tables (via COCONUT-SVSM's vTPM).

Intel reaches for measurement too. Its Hand-Off Block (HOB) is a proprietary
structure the guest TDVF reads to generate ACPI, and Intel recommends measuring
it into `RTMR[0]` (as OVMF does). Combined with the Arm CCA reference stack's
use of Devicetree, a VMM and confidential guest must support three different
platform-definition mechanisms — ACPI, HOB, Devicetree — each with a subtly
different risk, and the only way the industry has made any of them trustworthy
is to measure it.

### Dissolving the Tension: Validate, Don't Measure

A platform definition does not actually need to be measured to be safe — it
needs to be _validatable_. A description is data the guest can check: it can
reject overlapping regions or an implausible layout, reducing the worst case to
denial of service, which the host already holds by other means. The one thing
validation cannot tame is executable code, so the definition must also carry
none.

Devicetree satisfies both. It is a standardized format with safe Rust parsers
and battle-tested C implementations (libfdt), it is universally available on
`aarch64`, and it contains no executable code — no AML-equivalent — so the guest
can validate it unmeasured. On `x86` it could be translated to ACPI in-guest
(e.g. by firmware such as OVMF); Linux's direct `x86` Devicetree support is
limited but improving. PMI uses Devicetree as its platform-definition format on
every target; the trust model is the image author's choice (see below).

### Splitting Platform Definition from Resource Allocation

A DTB carries two concerns. Platform definition — MMIO map, interrupt
controller, transport choice — is image-owned: the kernel is built against it,
the host has no business changing it. Resource allocation — CPU count, memory,
NUMA topology — is host-owned: it varies per deployment and cannot be baked into
the image. One trust model for both leaves protection unused on one side or the
other.

PMI provides two extensions. [`direct`](direct.md) keeps the monolithic DTB
unchanged — host supplies, guest validates post-hoc. A hypervisor that already
drops a DTB at an IPA implements it for free; the cost is that validation cannot
fully ground-truth the image-owned half. [`merged`](merged.md) splits the
concerns: a measured base DTB for platform definition, plus an overlay
restricted to the resource-allocation allowlist (`/cpus`, `/memory@*`,
`/distance-map`, per-node `numa-node-id`); the guest validates and merges. The
cost is a hypervisor refactor — its DTB pipeline must drive through the
allowlist rather than around it — and the benefit is the full split.

Host resource decisions never enter the measurement; attestation depends on
image-controlled bytes alone — and PMI keeps that portable too. Early CC
implementations each measured the guest in their own idiomatic order, so each
VMM shipped its own measurement tool; PMI instead drives an explicit,
deterministic ordering of the guest image measurement (as IGVM demonstrated for
the narrower paravisor case). The result: the same PMI image yields the same
measurement on every VMM and under every host resource-allocation variation —
safe platform definition and portable attestation, without trading one for the
other.

## 3. Reuse of Existing Tooling and Formats

A new format must be re-tooled at every layer that touches it — producers,
loaders, verifiers, signers, inspectors. PMI avoids that by reusing standards
instead of inventing them: it extends PE rather than replacing it, and adopts
Devicetree for platform definition rather than a new encoding. Existing
ecosystems — `objcopy`, `sbsign`, `ukify`, UEFI loaders for PE; `dtc`, libfdt,
and safe Rust parsers for Devicetree — work unchanged.

Bespoke alternatives pay the opposite tax: a proprietary format like TDX's HOB,
or a whole new image format like IGVM, needs proprietary tooling everywhere it
is touched.

[amdsb]:
  https://www.amd.com/en/resources/product-security/bulletin/amd-sb-3012.html
