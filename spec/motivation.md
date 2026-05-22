# Motivation

PMI exists to solve four problems. Each problem has a one-to-one
corresponding goal in [Overview](overview.md). This document defines
the problems and explains why they are problems; the overview defines
the goals that solve them and the methods that deliver those goals.

| # | Problem (this document)                                | Goal (overview.md)                                                                                |
| - | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| 1 | Early boot can't defend against hypervisor attacks     | [Security against a malicious hypervisor](overview.md#security-against-a-malicious-hypervisor)    |
| 2 | One workload needs multiple image artifacts            | [Executable format portability](overview.md#executable-format-portability)                        |
| 3 | Same image, different VMM, different attestation       | [Attestation equivalence](overview.md#attestation-equivalence)                                    |
| 4 | Every new image format needs new tools at every layer  | [Tooling reuse](overview.md#tooling-reuse)                                                        |

## 1. Early boot can't defend against hypervisor attacks

Booting an operating system requires platform information: which
devices exist, where memory lives, what interrupt routing applies,
what CPU features are available. The kernel cannot synthesize this
information from nothing; it has to consume it from somewhere
upstream — Devicetree on many architectures, ACPI on x86. Whoever
supplies that information controls what the kernel believes about
its environment.

Early boot is the worst possible point in the kernel's life cycle to
validate what it consumes. The kernel is single-threaded, runs
without drivers, lacks network and stable storage, has no
cryptographic anchor to the underlying hardware (which in a VM
doesn't have one for the guest to anchor against), and cannot defer
the decision until it has more capability — every subsequent step
depends on having committed to a platform model. The kernel must
either trust what it is given or fail to boot.

On bare metal this trust is acceptable. Firmware is part of the
platform's identity, anchored through Secure Boot, measured boot,
TPM-attested firmware images, or equivalent vendor mechanisms; an
attacker who can substitute firmware has already won, so the kernel
treating firmware-provided platform information as trusted does not
expand the attack surface.

Under Confidential Computing the trust is not acceptable. The
hypervisor — the party supplying the platform information the guest
must consume — is explicitly outside the guest's trust boundary. The
hypervisor can supply any platform description it wants, and the
guest in early boot has neither the time nor the tooling to push
back. ACPI is the worst case here because AML is a Turing-complete
bytecode the kernel evaluates: a malicious DSDT runs arbitrary code
in kernel context, with full guest privileges, before any defensive
machinery has come up. The attack surface is concrete and
demonstrated:

- [AMD-SB-3012](https://www.amd.com/en/resources/product-security/bulletin/amd-sb-3012.html)
  — ACPI/AML injection in SEV guests via QEMU.
- [BadAML](https://dl.acm.org/doi/10.1145/3719027.3765123) (ACM CCS
  2025, Distinguished Paper) — universal AML injection across SEV and
  TDX guests.

The structural facts: the guest needs platform information to boot;
that information comes from the attacker; the guest's defensive
capability during the window when it must consume the information is
essentially zero.

**PMI's response:**
[Security against a malicious hypervisor](overview.md#security-against-a-malicious-hypervisor).

## 2. One workload needs multiple image artifacts

A single Linux workload — the same kernel, initrd, command line, and
service modules — is increasingly expected to run in many shapes:
bare metal under UEFI; as a direct-boot VM where the VMM extracts the
kernel; under guest firmware (OVMF) that loads the kernel from a
virtual disk; under a confidential VMM with a service module
(COCONUT-SVSM, a paravisor) that initializes the confidential
environment before the firmware sees it. The boot pipeline differs
per shape:

![Boot pipelines: bare metal versus modern VM](images/boot-modes.excalidraw.svg)

For an image author distributing a workload, the natural unit is
"one artifact." The artifact gets pulled from a registry, cached,
signed once, scanned once, attested once, and referenced by one
content hash. Anything that splits it into multiple artifacts
duplicates that whole pipeline. Real deployments span the spectrum:

- `qemu -kernel image.efi` — VMM extracts the kernel directly via
  the Linux boot protocol; no firmware involved.
- `qemu -bios OVMF.fd -kernel image.efi` — OVMF runs as guest UEFI
  and boots the kernel from the PE.
- `qemu -bios OVMF.fd -drive file=disk.img,...` — OVMF loads the
  kernel from a virtual disk; the PE need not carry one.
- COCONUT-SVSM + OVMF under SEV-SNP — the VMM launches the SVSM at
  VMPL0, which initializes the confidential environment, exposes a
  vTPM, and transitions OVMF to VMPL1.
- UEFI on bare metal via PXE or HTTP Boot — firmware fetches the PE
  remotely; the EFI stub boots the kernel.

Existing image formats are shape-specific. PE/UKI assumes UEFI loads
the image and runs an EFI stub. The Linux boot protocol assumes the
VMM extracts the kernel. IGVM (PMI's primary prior art) assumes a
paravisor-style confidential boot with measurement metadata. An
image author who wants to support more than one shape today produces
more than one artifact, with parallel build paths, parallel signing
flows, parallel registries to push to, and parallel rules to teach
deployers about which artifact to pull for which shape.

**PMI's response:**
[Executable format portability](overview.md#executable-format-portability).

## 3. Same image, different VMM, different attestation

Remote attestation lets a verifier (a third party, a key broker, a
policy engine) check what was launched before releasing secrets,
authorizing connections, or admitting the workload to a network. The
verifier checks a set of values from the attestation report against
expected values. The cryptographic centerpiece is a launch
measurement (SEV-SNP launch digest, CCA RIM, TDX MRTD) that is
supposed to be the identity of the workload — "this image launched
on conformant hardware" — and the verifier's purpose is to bind
release-of-secret to that identity.

For this to work, the launch measurement must be a function of the
image. The same image must produce the same measurement regardless
of where it ran, who ran it, or which conformant VMM submitted it to
the firmware. Otherwise the verifier is binding to an event ("this
specific launch under this specific VMM at this specific moment"),
not to a workload identity, and reproducibility breaks: a deployer
re-running the same image gets a different attestation, a verifier
re-validating last week's launch can't recompute the expected
measurement, and a tenant porting their workload from one cloud to
another loses the binding entirely.

Today the launch measurement is not a function of the image alone.
Several mechanisms drive divergence between two VMMs of the same
target running the same image:

- **Page submission order is implementation-defined.** A VMM that
  submits pages in disk-section order versus one that submits in
  ascending GPA order computes a different incremental hash from the
  same bytes.
- **The host picks values that are measured.** SEV's CPUID page and
  secrets-page placeholder, CCA's `RmiRealmParams` (SVE vector
  length, debug counts, hash algorithm), TDX's `ATTRIBUTES` and
  `XFAM` — each of these contributes to the launch measurement
  today, but the host decides what bytes they contain.
- **The host picks values that appear elsewhere in the report.** SEV
  launch policy, TDX `MRCONFIGID` / `MROWNER` / `MROWNERCONFIG`,
  CCA RPV — even when these don't enter the cryptographic hash,
  verifier policy checks them, and they vary per deployer.

Each divergence point is a place where the verifier's expected value
depends on knowledge the verifier doesn't have: which VMM
implementation, which host's configuration, which deployer's choice.

**PMI's response:**
[Attestation equivalence](overview.md#attestation-equivalence).

## 4. Every new image format needs new tools at every layer

Image formats are not consumed by one piece of software. A single
format gets touched by:

- **Producers** that build images from source artifacts (kernels,
  initrds, command lines, signatures).
- **Consumers** that load images and execute them (VMMs, in-guest
  pre-kernel stubs, UEFI firmware).
- **Verifiers** that reproduce the expected attestation from the
  image bytes.
- **Inspectors** that answer "what will this image do?" for
  debugging, auditing, registry display, and CI gating.
- **Signers** that bind a tenant or vendor identity to the artifact
  and producers/verifiers of those signatures.
- **Long-tail tooling** — strippers, disassemblers, registry layer
  introspectors — that touch images without understanding their full
  semantics.

Each of these layers represents accumulated engineering effort for
existing formats. PE has `objcopy`, `objdump`, `sbsign`, `pesign`,
`systemd-ukify`, `systemd-stub`, every UEFI loader, and decades of
hardening. A new image format that abandons PE forces every layer to
be re-implemented; bugs found in one stack don't fix the others, and
the new tooling lacks the operational maturity of what it replaced.

A new format that extends PE rather than replacing it inherits the
existing ecosystem for the layers that don't change, and only has to
introduce new tooling for the genuinely new concerns. But that new
tooling has its own constituency: a target-spec parser is wanted by
producers, VMMs, in-guest consumers, verifiers, and inspectors; a
DTBO merger is wanted by a small bootloader, a kernel-side
pre-handoff stub, and build-time validators; a tenant-identity
signer is wanted by image authors and deployers. If each new tool
is bound tightly to one application, the same conceptual work gets
re-implemented per consumer, and the format ends up with N tools
each doing one job badly instead of one tool doing the job well.

**PMI's response:**
[Tooling reuse](overview.md#tooling-reuse).
