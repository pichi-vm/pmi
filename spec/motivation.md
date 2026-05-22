# Motivation

PMI exists to solve six problems. Each problem has a one-to-one
corresponding goal in [Overview](overview.md). This document defines
the problems and explains why they are problems; the overview defines
the [categories](overview.md#categories) PMI distinguishes, the goals
that solve the problems, and the methods that deliver those goals.

| # | Problem (this document)                                | Goal (overview.md)                                                                                |
| - | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| 1 | One workload needs multiple image artifacts            | [Executable format portability](overview.md#executable-format-portability)                        |
| 2 | Existing solutions are target-specific and don't compose | [Uniform approach across targets](overview.md#uniform-approach-across-targets)                  |
| 3 | The host-controlled platform definition is a large attack surface | [Security against a malicious hypervisor](overview.md#security-against-a-malicious-hypervisor)    |
| 4 | Same image, different deployment, different measurement | [Measurement stability](overview.md#measurement-stability)                                        |
| 5 | Same image, different VMM, different attestation       | [Attestation equivalence](overview.md#attestation-equivalence)                                    |
| 6 | Every new image format needs new tools at every layer  | [Tooling reuse](overview.md#tooling-reuse)                                                        |

## 1. One workload needs multiple image artifacts

A single Linux workload — the same kernel, initrd, and command
line — is increasingly expected to run in many shapes:
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
VMM extracts the kernel. IGVM assumes a paravisor-style confidential
boot with measurement metadata. An image author who wants to support
more than one shape today produces more than one artifact, with
parallel build paths, parallel signing flows, parallel registries to
push to, and parallel rules to teach deployers about which artifact
to pull for which shape.

A service module (COCONUT-SVSM, paravisor) is a CC-specific
privileged component that initializes the confidential environment
and exposes services such as a vTPM before dropping the guest
firmware to a lower privilege level; it is absent from bare metal
and non-CC VM boot.

**PMI's response:**
[Executable format portability](overview.md#executable-format-portability).

## 2. Existing solutions are target-specific and don't compose

Each CC architecture and major hypervisor stack has independently
noticed that the problems below need solving, and each has rolled
its own answer:

- **Intel TDX defines HOB (Hand-Off Block)** as a per-target
  mechanism for the host to hand platform information to the guest.
  It standardizes the channel but not the trust model — the HOB
  remains host-controlled and the guest still has to validate it —
  and the mechanism only applies inside TDX.
- **IGVM (Microsoft)** is a hypervisor-anchored image format for
  paravisor-style confidential boots. It addresses a slice of the
  artifact-sprawl problem for that one boot shape, but doesn't
  extend to non-paravisor confidential boot, non-CC VM, bare metal,
  or to CC targets outside its scope.
- **QEMU has ad-hoc conventions** for SEV id-block / CPUID page /
  VMSA submission; libkrun does it differently; each cloud
  hypervisor has its own variant. None of them port.
- **Each VMM rolls its own measurement-reproduction tool** for its
  own targets. Each one has to be separately maintained, separately
  hardened, separately validated against attestation reports.
- **Each VMM exposes its own knobs** for binding the host's launch
  policy, choosing CPU feature exposure, configuring NUMA topology,
  signing tenant identity. Image authors learn N idiosyncratic
  surfaces.

Each of these is a target-specific or hypervisor-specific attempt at
the same general class of problem. The result: even where individual
problems are partly addressed, the solutions don't compose. A
workload that wants to run SEV on AWS, TDX on Azure, and CCA on a
tenant-owned bare-metal Arm box needs three different image-build
paths, three different verifier integrations, three different
consumer stubs. An image author supporting two CC architectures
effectively maintains two image-format toolchains in parallel.

**PMI's response:**
[Uniform approach across targets](overview.md#uniform-approach-across-targets).

## 3. The host-controlled platform definition is a large attack surface

Booting an operating system requires platform information: which
devices exist, where memory lives, what interrupt routing applies,
what CPU features are available. The kernel consumes this
information from somewhere upstream — Devicetree on many
architectures, ACPI on x86 — and uses it to shape essentially every
subsequent decision: which drivers to load, what memory regions are
usable, where MMIO accesses are routed, which CPU features are safe
to enable, which interrupts to handle.

The surface defined by that information is large. Devicetree alone
has thousands of bindings; ACPI adds AML, a Turing-complete bytecode
the kernel evaluates in its own context. The surface is also subtle:
many bindings express constraints whose violation manifests only as
later behavior (an off-by-one in a memory range, a slightly-wrong
interrupt-routing entry, a CPU feature bit asserted that the
underlying silicon does not actually implement). Comprehensively
validating an adversarial platform description at boot is not
intractable in principle, but it requires re-implementing the
semantics of every binding the kernel will rely on — a moving target
that grows with every new device class the kernel learns.

The party that supplies the platform description has two ways to
weaponize it. The direct route is malicious content: a DSDT whose
AML executes attacker-chosen code in kernel context; a device node
whose `compatible` string steers the kernel into a vulnerable
driver path; a CPU feature flag that asserts an extension the
silicon does not provide, taking the kernel down a misoptimized
code path. The indirect route is a legitimate-looking but
adversarially-chosen definition that expands the attack surface the
kernel exposes through its own subsequent behavior: exposing
devices known to have unpatched CVEs in their drivers; choosing
memory or interrupt layouts that interact badly with mitigations;
selecting CPU feature combinations that put the kernel on code
paths still being hardened. Even when no bytes are themselves
malicious, "the host gets to pick" is an asymmetric advantage.

On bare metal this asymmetry is acceptable. Firmware is part of the
platform's identity, anchored through Secure Boot, measured boot,
TPM-attested firmware images, or equivalent vendor mechanisms; an
attacker who can substitute firmware has already won. The kernel
treating firmware-provided platform information as a low-attention
input is consistent with the actual threat model.

Under Confidential Computing it is not acceptable. The hypervisor —
the party supplying the platform description — is explicitly outside
the guest's trust boundary, and every byte of the description
becomes attack-relevant. The attack surface is concrete and
demonstrated:

- [AMD-SB-3012](https://www.amd.com/en/resources/product-security/bulletin/amd-sb-3012.html)
  — ACPI/AML injection in SEV guests via QEMU.
- [BadAML](https://dl.acm.org/doi/10.1145/3719027.3765123) (ACM CCS
  2025, Distinguished Paper) — universal AML injection across SEV
  and TDX guests.

**PMI's response:**
[Security against a malicious hypervisor](overview.md#security-against-a-malicious-hypervisor).

## 4. Same image, different deployment, different measurement

A deployer running the same image at different sizes is doing
something operationally legitimate: more memory for a memory-hungry
workload, more vCPUs for a parallel one, an additional NUMA node
when the workload grows. These choices are about *how much* of a
platform, not *which* platform. The workload's identity hasn't
changed.

But the cryptographic launch measurement covers the platform
definition the workload sees. If the platform definition is treated
as a single monolithic thing, then resource-allocation values
(vCPU count, memory layout, NUMA topology) end up measured
alongside the fundamental hardware contract (devices, MMIO, IRQ
controller, PCIe topology). Scaling the deployment then perturbs
the measurement.

The distinction the verifier wants is between the two kinds of
platform information, illustrated by two questions:

- *If the deployer doubles memory and vCPUs, should the measurement
  change?* No — the workload is the same; the deployer is just
  sizing it.
- *If the deployer changes the location of an MMIO region or the
  interrupt controller version, should the measurement change?*
  Yes — that's a different platform contract, structurally a
  different workload.

Without an explicit separation between these two kinds of platform
information, the verifier has to know "what size was this deployed
at?" before it can recompute the expected measurement — defeating
the point of binding attestation to image identity. A deployer
scaling from 4 vCPUs to 8 vCPUs would produce a different
measurement than the 4-vCPU launch; a verifier that wants to bind
release of secrets to "this image identity, regardless of scaling"
cannot do it.

**PMI's response:**
[Measurement stability](overview.md#measurement-stability).

## 5. Same image, different VMM, different attestation

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
of which conformant VMM submitted it to the firmware. Otherwise the
verifier is binding to an event ("this specific launch under this
specific VMM"), not to a workload identity, and reproducibility
breaks: a verifier re-validating last week's launch can't recompute
the expected measurement, a tenant porting their workload from one
cloud to another loses the binding, and a deployer switching
hypervisor implementations breaks every prior attestation.

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

## 6. Every new image format needs new tools at every layer

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
