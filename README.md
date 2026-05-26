# PMI: Portable Machine Image

NOTE: this document is a working draft. Schemas and semantics may change.

## What is PMI?

PMI is a file format for distributing the early boot code (including a Linux
kernel, guest firmware, bootloader or a Confidential Computing service module)
of an operating system. It has the following goals:

1. portability across targets (bare metal, VM, AMD SEV, Arm CCA and Intel TDX).
2. portability of safe platform definition and attestation
3. reuse of existing tooling and formats

For more background on these goals, see [Motivation](spec/motivation.md).

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

| Prefix | Spec                       | Description                            |
| ------ | -------------------------- | -------------------------------------- |
| `vm`   | [spec/vm.md](spec/vm.md)   | Non-CC virtual machine target          |
| `sev`  | [spec/sev.md](spec/sev.md) | AMD SEV 3.0 (SEV-SNP) confidential VMs |
| `tdx`  | [spec/tdx.md](spec/tdx.md) | Intel TDX confidential VMs (draft)     |
| `cca`  | [spec/cca.md](spec/cca.md) | Arm CCA confidential VMs (draft)       |

To register a new extension, open an issue or pull request against the PMI spec
repository. Be sure to follow the format in the existing extensions.

## Examples

Each target spec ends with a concrete CBOR walkthrough of a PMI image for that
target.
