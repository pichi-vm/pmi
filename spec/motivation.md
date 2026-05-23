# Motivation

PMI exists to solve three problems. Each problem has a one-to-one
corresponding goal in [Overview](overview.md). This document defines
the problems and explains why they are problems; the overview defines
the goals that solve them and the methods that deliver those goals.

| # | Problem (this document)                                  | Goal (overview.md)                                                                  |
| - | -------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| 1 | One workload needs multiple image artifacts              | [Executable format portability](overview.md#executable-format-portability)          |
| 2 | Existing solutions are target-specific and don't compose | [Uniform approach across targets](overview.md#uniform-approach-across-targets)      |
| 3 | Every new image format needs new tools at every layer    | [Tooling reuse](overview.md#tooling-reuse)                                          |

PMI is intentionally narrow. Higher-level concerns — platform
identity, host-conformance checking, attestation policy, the
measured-vs-unmeasured boundary as a security argument — are the
business of upper-layer specs (e.g., dillo) that build on top of
PMI. PMI provides the substrate: the PE container, target-specific
CBOR launch recipes, the action mechanism that drives firmware ABIs,
and a namespacing rule for upper-layer extensions.

## 1. One workload needs multiple image artifacts

A single Linux workload — the same kernel, initrd, and command
line — is increasingly expected to run in many shapes: bare metal
under UEFI; as a direct-boot VM where the VMM extracts the kernel;
under guest firmware (OVMF) that loads the kernel from a virtual
disk; under a confidential VMM with a service module (COCONUT-SVSM,
a paravisor) that initializes the confidential environment before
the firmware sees it. The boot pipeline differs per shape:

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
noticed that VM launch needs a launch recipe — a way to tell the
VMM how to assemble the guest, what bytes go where, what firmware
calls to make, in what order — and each has rolled its own answer:

- **Intel TDX defines HOB (Hand-Off Block)** as a per-target
  mechanism for the host to hand platform information to the guest.
  The mechanism only applies inside TDX.
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
- **Each VMM exposes its own knobs** for choosing CPU feature
  exposure, configuring NUMA topology, signing tenant identity.
  Image authors learn N idiosyncratic surfaces.

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

## 3. Every new image format needs new tools at every layer

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
tenant-identity signer is wanted by image authors and deployers. If
each new tool is bound tightly to one application, the same
conceptual work gets re-implemented per consumer, and the format
ends up with N tools each doing one job badly instead of one tool
doing the job well.

**PMI's response:**
[Tooling reuse](overview.md#tooling-reuse).
