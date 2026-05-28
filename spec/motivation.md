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

A hypervisor needs four inputs to start a guest:

1. the CPU features the guest will see,
2. the CPU's initial register state,
3. the platform description — memory map, vCPU count, interrupt controller, PCI host, power button,
4. the bytes the guest first executes — Linux, OVMF, COCONUT-SVSM.

In a bare-metal world platform firmware decides all four. Virtualization
inherited that shape: the host decides, the guest accepts. The inheritance is
engineering inertia, not a requirement.

Under Confidential Computing it becomes risk. Each of the four is a channel by
which a malicious host can attack the guest. AMD and Intel reach for the same
defense: measure the platform description so tampering surfaces at attestation
— ACPI tables into a COCONUT-SVSM vTPM, the Hand-Off Block into `RTMR[0]`. But
measurement binds host decisions into the guest's identity for good: the same
image then attests differently on every hypervisor, every memory size, every
minor VMM version bump. Safety bought this way costs portable attestation.

That trap rests on an unexamined premise — that the host has to make these
choices at all. It does not. The host has no real reason to choose the guest's
CPU features, its initial register state, or the bytes at its reset vector;
historically it did because firmware did, not because the choice belonged to
it. PMI moves all four declarations into the image. The host delivers them or
refuses to launch.

Three of the four invert cleanly. The fourth, the platform description, has
one slice the host genuinely owns: resource allocation. CPU count, memory
size, and NUMA topology vary per deployment and cannot be baked into an image.
PMI splits the platform description by trust model. Platform definition —
image-owned — moves into the image. Resource allocation arrives via a
Devicetree overlay (see [`merged`](merged.md)) restricted to an allowlist the
guest validates.

The defense against a malicious description is validation, not measurement —
and Devicetree is what makes that possible, because it carries no executable
code. It has standardized parsers in safe Rust and in libfdt, is universally
available on aarch64, and the guest validates it directly. On x86 it can be
translated to ACPI in-guest by firmware such as OVMF; direct kernel support is
improving.

Image-controlled bytes alone determine the measurement; host hardware, host
resources, host VMM version — none of them appear. **Every compliant VMM
produces byte-identical measurements from the same PMI image.** Every
extension that participates in a target's launch measurement MUST preserve
this invariant.

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
