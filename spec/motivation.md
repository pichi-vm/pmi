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

A single Linux workload, the same kernel, initrd, and command line,
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

PMI is one image that boots every shape (bare metal under UEFI, a standard VM,
or a confidential VM) from the same bytes.

## 2. Portable, Safe Platform Definition and Attestation

A hypervisor needs four inputs to start a guest:

1. the CPU features the guest will see,
2. the CPU's initial register state,
3. the platform description: memory map, vCPU count, interrupt controller, PCI host, power button,
4. the bytes the guest first executes: Linux, OVMF, COCONUT-SVSM.

In a bare-metal world platform firmware decides all four. Virtualization
inherited that shape: the host decides, the guest accepts. The inheritance is
engineering inertia rather than a requirement.

Under Confidential Computing it becomes risk. Each of the four is a channel by
which a malicious host can attack the guest. AMD and Intel reach for the same
defense: measure the platform description so tampering surfaces at attestation,
with ACPI tables going into a COCONUT-SVSM vTPM and the Hand-Off Block into
`RTMR[0]`. But measurement binds host decisions into the guest's identity for
good: the same image then attests differently on every hypervisor, every memory
size, every minor VMM version bump. Safety bought this way costs portable
attestation.

That trap rests on an unexamined premise, that the host has to make these
choices at all. It does not. The host has no real reason to choose the guest's
CPU features, its initial register state, the bytes at its reset vector, or the
platform it runs on: where its devices live, which interrupt controller it has,
how its transports are wired. Historically it did because firmware did, not
because the choice belonged to it. PMI moves all four declarations into the
image. The host delivers them or refuses to launch.

### Splitting platform definition from resource allocation

The first three inputs invert outright: the image states them and the host has
nothing to add. The fourth, the platform description, is the only one with a
slice the host can own, namely _resource allocation_: CPU count, memory size,
and NUMA topology typically vary per deployment. So PMI splits the platform
description by trust model and inverts the half that admits it:

- **Platform definition** (the device MMIO map, interrupt controller, transport
  choice, and device topology) is image-owned. The host does not describe it to
  the guest; the _image_ declares it, and the host must instantiate a VM that
  matches or refuse to launch. The guest reads its platform from the measured
  image, never from the host.
- **Resource allocation** (CPU instances, memory, NUMA distances) may arrive
  from the host via a Devicetree overlay (see [`dt`](dt.md)), restricted
  to an allowlist the guest validates. An image that wants an exact layout may
  instead fix CPUs and memory in the measured base; NUMA affinity, being a
  host placement decision, stays with the host whenever an overlay is present.

This is not the legacy model, in which the host enumerates its own devices and
the guest adapts to whatever it is handed. Under PMI the host cannot relocate or
substitute the guest's platform; its only power over platform definition is
refusal.

The split lets each half use the right defense. The image-owned platform
definition folds into the launch measurement like any other image byte; the
host-supplied overlay is validated against the allowlist at boot. Neither
folds host resource decisions into the guest's identity. Devicetree is what
makes both moves possible, because it carries no executable code. ACPI's AML
would force the guest to trust whatever the host hands it, which is the gap
AMD and Intel close by measuring the description wholesale. Devicetree has
standardized parsers in safe Rust and in libfdt, is universally available on
aarch64; on x86 it can be translated to ACPI in-guest by firmware such as
OVMF, and direct kernel support is improving.

Image-controlled bytes alone determine the measurement; host hardware, host
resources, and host VMM version never appear. Every compliant VMM produces
byte-identical measurements from the same PMI image. Every extension that
participates in a target's launch measurement MUST preserve this invariant.

## 3. Reuse of Existing Tooling and Formats

A new format must be re-tooled at every layer that touches it: producers,
loaders, verifiers, signers, inspectors. PMI avoids that by reusing standards
instead of inventing them: it extends PE rather than replacing it, and adopts
Devicetree for platform definition rather than a new encoding. Existing
ecosystems work unchanged: `objcopy`, `sbsign`, `ukify`, and UEFI loaders for
PE; `dtc`, libfdt, and safe Rust parsers for Devicetree.

Bespoke alternatives pay the opposite tax: a proprietary format like TDX's HOB,
or a whole new image format like IGVM, needs proprietary tooling everywhere it
is touched.
