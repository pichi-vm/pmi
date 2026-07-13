# PMI: Portable Machine Image

NOTE: this document is a working draft. Schemas and semantics may change.

## What is PMI?

PMI is a file format for distributing an operating system's early boot code:
Linux kernels, guest firmware, bootloaders, paravisors, and Confidential
Computing service modules. It has the following goals:

1. portability across targets (bare metal, VM, AMD SEV, Arm CCA and Intel TDX).
2. portability of safe platform definition and attestation
3. reuse of existing tooling and formats

For more background on these goals, see [Motivation](spec/motivation.md).

## Why does PMI exist?

Today's Confidential Computing offerings (AMD SEV-SNP, Intel TDX, Arm CCA)
inherit a structure from bare metal: the hypervisor chooses much of what the
guest sees — its CPU features, its initial register state, its platform
description, and the bytes it first executes. That inheritance is engineering
inertia, not a security requirement, and France's national cybersecurity agency
named the cost directly in its October 2025 [Technical Position Paper on
Confidential Computing][anssi-cc]:

> Part of the code running in the User-provided TCB is often injected by the
> cloud-provider, especially in the case of cVMs: the firmware responsible for
> the early stages of booting the VM, as well as the virtual TPM used to attest
> the later boot stages, are typically beyond the control of the user. […]
> Attestation of every step of the bootchain is necessary to verify that the
> entire User-provided TCB has not been compromised by an admin attack, but it
> is impossible to perform on current cloud offerings for confidential VMs.
>
> — ANSSI, _Technical Position Paper on Confidential Computing_, v1.0, October
> 2025

PMI moves those declarations into the image. The host delivers them or refuses
to launch. One artifact, one measurement, full-chain attestation that does not
depend on which cloud or hypervisor provisioned the VM or how many resources are
given to it.

## What does a PMI image contain?

PMI is a container; the payload is your choice. Some possible compositions:

- **`PMI(Linux)`** — a kernel, initrd, and command line, direct-boot. Replaces
  `qemu -kernel` provisioning and the parallel UKI build.
- **`PMI(OVMF, Linux)`** — UEFI guest firmware plus a Linux UKI. The same file
  boots as a UKI on bare metal — where the platform's own UEFI runs it and the
  embedded OVMF is unused — and under that embedded OVMF as guest firmware in a
  VM. Both paths boot the same kernel from one artifact.
- **`PMI(SVSM, OVMF, Linux)`** — a confidential VM with COCONUT-SVSM as the
  service module providing an in-enclave vTPM, OVMF as guest firmware, and Linux
  on top. Three components previously provisioned and attested separately,
  shipped as one image.
- **`PMI(OpenHCL)`** — Microsoft's OpenHCL paravisor as the guest payload.

Each composition is one file. Bare metal, hypervisor, and confidential hardware
read the same bytes, and the launch measurement is byte-identical across every
compliant VMM.

## How does it work?

PMI is a standard Portable Executable, just like a Linux UKI. A compliant VM
implementation will choose a **target**, read the CBOR document in the
`.pmi.<target>` section and follow each of the ordered **actions** defined in
the document. This allows a single PE executable to boot on bare metal (i.e. as
a UKI) or on a VM/CVM using the PMI extensions. For efficient zero-copy loading,
PMI imposes [page granularity](spec/granularity.md) rules on its sections. PMI
has an extremely simple [core specification](spec/core.md) which defines the
format of the **target** CBOR document and two simple **actions**. Most
functionality is defined as [extensions](spec/extensions.md); see the extension
registry below.

## Extension Registry

The following extensions are registered with PMI.

| Prefix   | Spec                             | Description                                 |
| -------- | -------------------------------- | ------------------------------------------- |
| `vm`     | [spec/vm.md](spec/vm.md)         | Non-CC virtual machine target               |
| `sev`    | [spec/sev.md](spec/sev.md)       | AMD SEV 3.0 (SEV-SNP) confidential VMs      |
| `tdx`    | [spec/tdx.md](spec/tdx.md)       | Intel TDX confidential VMs (draft)          |
| `cca`    | [spec/cca.md](spec/cca.md)       | Arm CCA confidential VMs (draft)            |
| `dt`     | [spec/dt.md](spec/dt.md)         | Image base DTB + host overlay (split-trust) |
| `cpu`    | [spec/cpu.md](spec/cpu.md)       | vCPU ISA baseline                           |

To register a new extension, open an issue or pull request against the PMI spec
repository. Be sure to follow the format in the existing extensions.

[anssi-cc]:
  https://messervices.cyber.gouv.fr/documents-guides/anssi-technical-position-paper-coco-v1.0.pdf
