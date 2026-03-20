# PMI: Portable Machine Image

## Introduction

Today there is no single image format that can boot a machine across all
deployment contexts — bare metal, virtual machines, and confidential virtual
machines — while giving tenants control over what software they trust. The
ecosystem uses PE for UEFI boot, UKI for direct kernel boot, and IGVM for
confidential computing, requiring separate build pipelines and tooling for each.

PMI solves this by extending the PE format with a `.pmi` section containing a
CBOR-encoded manifest — the complete recipe for launching a guest. The VMM reads
the manifest, filters by platform, and executes the instructions in order. No
firmware introspection, no hardcoded conventions, no implicit knowledge.

A PMI without a `.pmi` section is a standard PE binary (or UKI, if it contains a
kernel). A PMI without a `.linux` section is a firmware-only image. The format
does not prescribe contents — it instructs the VMM on what to do with whatever
is there.

## Boot modes

A machine boots by combining three components, each of which may be absent,
provided by the host, provided by the tenant (bundled in the image), or loaded
from disk:

| Service | Firmware |  Kernel   | Example                                     | Description                                               |
| :-----: | :------: | :-------: | :------------------------------------------ | :-------------------------------------------------------- |
|         |          |  bundled  | systemd-boot, PXE, UEFI HTTP boot           | Bare metal, UEFI executes PE directly                     |
|         |          | extracted | QEMU `-kernel`, cloud-hypervisor `--kernel` | VMM extracts kernel from PE, no firmware                  |
|         |   host   | extracted | QEMU `-bios OVMF.fd -kernel bzImage`        | Host firmware, VMM extracts kernel and passes to firmware |
|         |   host   |  bundled  | QEMU `-bios OVMF.fd`, UKI via fw_cfg        | Host firmware, UKI passed to guest firmware               |
|         |  tenant  |  bundled  |                                             | Tenant firmware from image, UKI passed to guest firmware  |
|         |   host   |  on disk  | Standard VM: QEMU + OVMF + guest disk       | Host firmware, kernel from disk                           |
|         |  tenant  |  on disk  | cloud-hypervisor `--firmware` + guest disk  | Tenant firmware from image, kernel from disk              |
|  host   |   host   |  on disk  | Azure CVM (Microsoft paravisor)             | Host service + host firmware, kernel from disk            |
| tenant  |  tenant  |  on disk  | COCONUT-SVSM + OVMF via IGVM                | Tenant service + tenant firmware, kernel from disk        |

## Format comparison

No existing format covers all of these modes:

- **PE** is the universal UEFI boot image, but has no virtualization or
  confidential computing semantics.
- **UKI** (PE + kernel + EFI stub) adds VMM direct boot support via the Linux
  boot protocol, but cannot carry firmware, service modules, or CC metadata.
- **IGVM** provides full CC semantics and can carry firmware and service
  modules, but is not a PE — it cannot boot on bare metal, via PXE, or via UEFI
  HTTP boot.

| Boot mode                                      | PE  | UKI | IGVM | PMI |
| :--------------------------------------------- | :-: | :-: | :--: | :-: |
| **Bare metal**                                 |  ✓  |  ✓  |      |  ✓  |
| **Direct** (CC optional)                       |     |  ✓  |  ✓   |  ✓  |
| **Indirect** (firmware boots kernel from disk) |     |     |  ✓   |  ✓  |
| **Serviced** (CC, service module at layer 0)   |     |     |  ✓   |  ✓  |

PMI is a strict superset. It inherits bare metal and direct boot from UKI, adds
every capability IGVM provides, and remains a valid PE throughout. A single
build pipeline produces one artifact for all deployment targets.

## Why not IGVM?

IGVM is a well-designed format for its original purpose: describing confidential
guest images for VMMs. PMI exists because that purpose is too narrow.

**IGVM cannot boot on bare metal.** IGVM is not a PE. UEFI firmware cannot load
it. PXE cannot chainload it. HTTP Boot cannot fetch and execute it. Any
deployment that touches real hardware needs a separate image and a separate
build pipeline. PMI is a PE — the same artifact boots on bare metal, in a VM,
and in a confidential VM.

**IGVM encodes page-level load commands.** An IGVM file is a sequence of
directives: "load these bytes at this GPA as this page type." The producer must
decide page granularity, interleave measured and unmeasured pages in the correct
order, and emit one directive per page or contiguous range. The VMM is tightly
coupled to this page-level instruction stream. PMI expresses regions — the VMM
decides how to map them.

**IGVM conflates data and policy.** IGVM directives mix guest memory contents,
page types, measurement boundaries, and platform policy into a single ordered
stream. Changing the SNP policy means re-serializing the directive stream.
Adding a new section means inserting directives at the correct position among
platform-specific pages. PMI separates these concerns: sections carry data,
parameters carry runtime state, and platform config carries policy. Each can
change independently.

**IGVM requires format-aware tooling.** You cannot inspect or modify an IGVM
file with standard PE tools. objcopy, readelf, sbsign, and systemd-ukify do not
apply. PMI images are PE files — every tool in the PE ecosystem works
unmodified.

**IGVM cannot carry a bootable kernel.** IGVM has no equivalent of the UKI
model: an EFI stub, kernel, initrd, and command line in a single signed PE that
UEFI can execute. To boot a kernel with IGVM, you must either bundle firmware
that loads the kernel from disk or maintain a separate UKI for non-CC contexts.
PMI inherits UKI's direct boot model and adds CC on top.

**IGVM is x86-centric in practice.** The format was designed around SNP and TDX
page types and measurement semantics. Arm CCA support requires mapping its
concepts onto IGVM's directive model. PMI treats platforms as peers — each gets
a config block with its own binding specification, no mapping required.

PMI does not replace IGVM for contexts where IGVM is sufficient. But when a
single artifact must serve bare metal, virtual, and confidential virtual
deployment — or when the build pipeline should not need to understand page-level
CC semantics — PMI is the simpler path.

## Design principles

- **PE is the container.** UEFI, PXE, HTTP boot, systemd-boot, and VMMs all
  already consume PE. No new container format is needed.
- **The manifest is the VMM's instruction set.** The VMM reads the manifest and
  executes it. It does not introspect firmware binaries, rely on hardcoded
  conventions, or make assumptions about image contents. Everything the VMM
  needs to know is in the manifest.
- **Sections then parameters.** Sections are loaded first (in array order), then
  parameters are populated (in array order). Sections carry measured tenant data
  from the PE. Parameters carry unmeasured VMM-generated runtime data that may
  reference section memory. Measurement follows section order.
- **Platform config owns platform actions.** Sections and parameters are
  platform-neutral. Platform-specific actions (guest policy, VMSAs, secrets
  pages, CPUID pages, TD HOBs, RECs) live in the platform config and are
  executed at defined points around the section/parameter walk — never
  interleaved with it.
- **CC is additive.** A PMI without a manifest is a UKI. A PMI with a manifest
  boots identically on non-CC VMMs that ignore the manifest. CC semantics are
  layered on top, never required.
- **Platform-neutral.** AMD SEV 3.0, Intel TDX, and Arm CCA are equivalent from
  the format's perspective — different `platform` strings, same schema. New
  platforms require no schema changes.
- **Extensible everywhere.** Every structure accepts unknown keys. Sections,
  parameters, and platform config can all carry additional data without schema
  changes.
- **Host decides page granularity.** The manifest expresses regions not pages.
  The host maps them as 4K, 2M, or 1G.
- **Toolchain compatible.** mkosi, systemd-ukify, sbsign, objcopy all work on
  PMI images. Standard PE tooling adds non-loaded sections for CC metadata
  without invalidating the UKI.

## PE container

A PMI is a valid PE with the following constraints:

- **The manifest is stored in a `.pmi` PE section.** The section is non-loaded
  (`IMAGE_SCN_MEM_DISCARDABLE`). UEFI firmware and standard PE loaders ignore
  it. VMMs that understand PMI read it.

- **Section names must fit in 8 bytes.** The PE `IMAGE_SECTION_HEADER.Name`
  field is a fixed 8-byte array. PMI does not use the COFF string table
  extension. Names shorter than 8 bytes are null-padded; names of exactly 8
  bytes have no null terminator.

- **Sections ≥ 2M must have `PointerToRawData` aligned to 2M.** This allows the
  VMM to mmap the file and pass large sections directly to platform APIs (e.g.,
  `SNP_LAUNCH_UPDATE`) using 2M pages with no copy.

- **Sections < 2M must have `PointerToRawData` aligned to 4K.** This ensures 4K
  zero-copy for smaller sections.

Tools which build PMIs MUST follow these alignment requirements. Tools which
consume PMIs MAY reject images that do not conform to them.

The VMM reads `PointerToRawData` and `SizeOfRawData` from the PE section
headers. If the offset is 2M-aligned and the size is ≥ 2M, the VMM can use 2M
pages. Otherwise it uses 4K pages. Zero-copy in both cases.

## Manifest schema

```cddl
; ================================================================
; Extensibility convention
; ================================================================
;
; Every PMI-defined map (manifest, section, parameter, platforms)
; accepts additional keys beyond those defined here. Well-known
; keys are short, unnamespaced strings (e.g., "name", "measured",
; "snp"). Extension keys MUST use a collision-resistant namespaced
; form: "namespace:key" (e.g., "vendor:feature"). Extension values
; for enumerated types (e.g., param-type) and platform keys in the
; platforms map follow the same convention.
;
; Platform-specific structures (snp-config, tdx-config, etc.) are
; defined by their respective platform binding specifications.
; PMI does not add extension points to structures it does not own.
;
; Consumers MUST ignore keys and types they do not recognize.

; ================================================================
; Manifest
; ================================================================

manifest = {
  "version"      => uint,                ; schema version, currently 1
  "sections"     => [+ section]
  ? "parameters"  => [+ parameter]
  ? "platforms"   => platforms
  * tstr => any,                        ; extension point
}
```

Example — a minimal manifest for direct boot with CC:

```cbor-diag
{
  "version": 1,
  "sections": [
    {"name": ".linux"},
    {"name": ".initrd"},
    {"name": ".cmdline"},
    {"name": ".acpi"},
    {"name": ".bprms"}
  ],
  "parameters": [
    {"type": "e820", "address": 8192, "size": 4096}
  ],
  "platforms": {
    "snp": {
      "policy": {"smt": true, "migrate-ma": false, "debug": false},
      "vmsa": ".vmsa"
    },
    "native": {}
  }
}
```

### Sections

Ordered. The VMM loads sections in array order (step 4). Measurement follows
the same order.

Each section references a PE section by name. The VMM reads `VirtualAddress`,
`SizeOfRawData`, and `PointerToRawData` from the PE section header. If
`VirtualSize` > `SizeOfRawData`, the remainder is zero-filled (standard PE .bss
behavior — this is how reserved memory regions are expressed without file
backing).

Not all PE sections appear in this array. Platform configs may reference
additional PE sections by name (e.g., SNP's `"vmsa"`, `"secrets"`, `"cpuid"`).
Those sections are loaded by the platform adapter during steps 3 or 6, not
during step 4.

```cddl
section = {
  "name"         => tstr,               ; PE section name (e.g., ".ovmf", ".svsm")
  ? "platforms"   => [+ tstr],            ; keys from the platforms map
  ? "measured"    => bool,              ; default true
  * tstr => any,                        ; extension point
}
```

### Parameters

Ordered. The VMM populates parameters in array order (step 5). Parameters are
VMM-generated data deposited at a specified GPA. They may reference memory
established by prior sections.

```cddl
parameter = {
  "type"         => param-type
  "address"      => uint,               ; guest physical address
  "size"         => uint,               ; in bytes
  * tstr => any,                        ; extension point
}

; Well-known parameter types. Extensions use "namespace:type".
param-type = "e820"                     ; x86 e820 memory map (array of e820_entry)
           / "rsdp"                     ; ACPI Root System Description Pointer
           / "srat"                     ; ACPI System Resource Affinity Table
           / "madt"                     ; ACPI Multiple APIC Description Table
           / tstr                       ; extension point ("namespace:type")
```

### Platforms

Platform-specific configuration. The platform config drives steps 2, 3, 6, and
7 of the execution model. The VMM's platform adapter reads the keys it
understands and ignores the rest.

Well-known platform types are defined below. Extensions use `"namespace:type"`
(e.g., `"myvendor:custom-tee"`).

```cddl
platforms = {
  ? "snp"     => snp-config
  ? "tdx"     => tdx-config
  ? "cca"     => cca-config
  ? "native"  => native-config
  * tstr => any,                        ; extension platform types
}

; --- AMD SEV 3.0 (SNP) ---
;
; The "secrets", "cpuid", and "vmsa" fields reference PE sections
; by name. The SNP adapter loads them during step 6 (post-load)
; via SNP_LAUNCH_UPDATE with the corresponding page type, in the
; order: secrets, cpuid, vmsa.
;
; - secrets: section is .bss-style; VMM generates content at launch.
; - cpuid: section is .bss-style; VMM generates content at launch.
; - vmsa: section data is the 4K VMPL0 BSP register state.

snp-config = {
  ? "policy"    => snp-policy,          ; step 2: SNP_LAUNCH_START (default: all-zero)
  ? "secrets"   => tstr,               ; step 6: PE section, page_type=secrets
  ? "cpuid"     => tstr,               ; step 6: PE section, page_type=cpuid
  "vmsa"        => tstr,               ; step 6: PE section, page_type=vmsa
}

; See SEV-SNP Firmware ABI §4.3, Table 9.
snp-policy = {
  ? "abi"                    => snp-abi,  ; [15:0] minimum ABI version
  ? "debug"                  => bool,    ; [19] allow debugging
  ? "migrate-ma"             => bool,    ; [18] allow migration agent association
  ? "single-socket"          => bool,    ; [20] restrict to single socket
  ? "cxl-allow"              => bool,    ; [21] allow CXL devices/memory
  ? "mem-aes-256-xts"        => bool,    ; [22] require AES-256-XTS memory encryption
  ? "rapl-dis"               => bool,    ; [23] require RAPL disabled
  ? "ciphertext-hiding-dram" => bool,    ; [24] require ciphertext hiding for DRAM
  ? "page-swap-disable"      => bool,    ; [25] disable SNP_PAGE_MOVE/SWAP_OUT/SWAP_IN
  ? "smt"                    => bool,    ; [16] allow SMT
}

snp-abi = {
  ? "major" => uint,                     ; [15:8] minimum ABI major version
  ? "minor" => uint,                     ; [7:0] minimum ABI minor version
}

; --- Intel TDX ---
; TODO: define when TDX binding is specified.

tdx-config = {
}

; --- Arm CCA ---
; TODO: define when CCA binding is specified.

cca-config = {
}

; --- Native (no CC) ---

native-config = {
}
```

## VMM execution model

The VMM processes the manifest in eight steps:

1. **Select platform.** Read the `platforms` map, select the entry matching the
   current CC platform (or `"native"` for non-CC).

2. **Platform initialize.** Call the platform's initialization API with
   parameters from the selected platform config. This creates the platform's
   cryptographic context and must complete before any data is loaded into the
   guest.

3. **Platform pre-load.** Execute platform-specific actions that the platform
   binding defines as preceding the section/parameter walk.

4. **Load sections.** Iterate the `sections` array in order. For each entry,
   skip if `platforms` is present and does not include the current platform.
   Otherwise, read the named PE section and load it into guest memory at its
   `VirtualAddress`. If `measured` is true (the default), feed the data to the
   platform's measurement API.

5. **Populate parameters.** Iterate the `parameters` array in order. For each
   entry, allocate a region at the specified GPA and populate it with
   VMM-generated data. Parameters may reference memory established by prior
   sections (e.g., an e820 map describing the address ranges loaded in step 4).

6. **Platform post-load.** Execute platform-specific actions that the platform
   binding defines as following the section/parameter walk.

7. **Platform finalize.** Call the platform's finalization API. This seals the
   measurement and marks the guest ready for execution.

8. **Start the guest.**

Steps 2, 3, 6, and 7 are all derived from the platform config. PMI defines their
position in the sequence but not their contents — each platform's binding
specification defines what actions occur at each step and in what order. Steps 4
and 5 are controlled by the manifest's `sections` and `parameters` arrays
respectively.

### Platform binding summary

| Step          | AMD SEV 3.0                            | Intel TDX                             | Arm CCA                           |
| ------------- | -------------------------------------- | ------------------------------------- | --------------------------------- |
| 2. Initialize | `SNP_LAUNCH_START` (policy)            | `KVM_TDX_INIT_VM` (attributes, xfam)  | `RMI_REALM_CREATE` (realm params) |
| 3. Pre-load   | (none)                                 | `KVM_TDX_INIT_VCPU`                   | `RMI_RTT_CREATE`                  |
| 4. Sections   | `SNP_LAUNCH_UPDATE` per section        | `KVM_TDX_INIT_MEM_REGION` per section | `RMI_DATA_CREATE` per section     |
| 5. Parameters | populate at GPA                        | populate at GPA                       | populate at GPA                   |
| 6. Post-load  | `.secrets`, `.cpuid`, `.vmsa` sections | TD HOBs                               | `RMI_REC_CREATE`                  |
| 7. Finalize   | `SNP_LAUNCH_FINISH` (id_block)         | `KVM_TDX_FINALIZE_VM`                 | `RMI_REALM_ACTIVATE`              |

This table is informative. The normative definitions belong in each platform's
binding specification, not in PMI.

### Execution walkthrough: direct boot with CC (AMD SEV 3.0)

Using the manifest from the schema example above:

**SEV 3.0 (steps 1–8):**

1. Select `"snp"`.
2. `SNP_LAUNCH_START` with policy.
3. (No pre-load for SNP.)
4. Load sections: `SNP_LAUNCH_UPDATE` for `.linux`, `.initrd`, `.cmdline`,
   `.acpi`, `.bprms` (all measured).
5. Populate e820 at its GPA (unmeasured).
6. Post-load: load `.vmsa` (page_type=vmsa) via `SNP_LAUNCH_UPDATE`.
7. `SNP_LAUNCH_FINISH`.
8. Kernel starts.

**Native:** Steps 2, 3, 6, 7 are no-ops. VMM loads sections, populates
parameters, sets registers to kernel entry, starts guest.

**Bare metal:** UEFI ignores `.pmi`. EFI stub in `.linux` executes normally.
Standard UKI boot.

### Execution walkthrough: SVSM + OVMF (AMD SEV 3.0)

```cbor-diag
{
  "version": 1,
  "sections": [
    {"name": ".svsm", "platforms": ["snp"]},
    {"name": ".ovmf"},
    {"name": ".linux"},
    {"name": ".initrd"},
    {"name": ".cmdline"},
    {"name": ".osrel"}
  ],
  "parameters": [
    {"type": "e820", "address": 8192, "size": 4096}
  ],
  "platforms": {
    "snp": {
      "policy": {"smt": true, "migrate-ma": false, "debug": false},
      "secrets": ".secrets",
      "cpuid": ".cpuid",
      "vmsa": ".vmsa"
    },
    "native": {}
  }
}
```

**SEV 3.0 (steps 1–8):**

1. Select `"snp"`.
2. `SNP_LAUNCH_START` with policy.
3. (No pre-load for SNP.)
4. Load sections: `SNP_LAUNCH_UPDATE` for `.svsm`, `.ovmf`. Feed measured
   sections to measurement API. Skip `.linux`/`.initrd`/`.cmdline`/`.osrel` if
   doing indirect boot (OVMF boots kernel from disk).
5. Populate e820 (unmeasured).
6. Post-load: load `.secrets` (page_type=secrets, VMM-generated), `.cpuid`
   (page_type=cpuid, VMM-generated), `.vmsa` (page_type=vmsa) via
   `SNP_LAUNCH_UPDATE`.
7. `SNP_LAUNCH_FINISH`.
8. SVSM starts at VMPL0, initializes vTPM, creates VMPL1 VMSA for OVMF,
   transitions OVMF. OVMF boots kernel from disk, measures boot via SVSM vTPM.

**Native (steps 1–8):**

1. Select `"native"`.
2. (No-op.)
3. (No-op.)
4. Skip `.svsm` (filtered). Load `.ovmf`.
5. Populate e820.
6. (No-op.)
7. (No-op.)
8. Set registers to OVMF reset vector. OVMF boots kernel from disk.

**Bare metal:** UEFI ignores `.pmi`, `.svsm`, `.ovmf` (non-loaded sections). EFI
stub in `.linux` executes normally. Standard UKI boot.

One artifact. One manifest. Three execution paths.

### Per-component ACPI

```cbor-diag
{
  "version": 1,
  "sections": [
    {"name": ".svsm", "platforms": ["snp"]},
    {"name": ".acpi0", "platforms": ["snp"]},
    {"name": ".acpi1"},
    {"name": ".ovmf"},
    {"name": ".linux"},
    {"name": ".initrd"},
    {"name": ".cmdline"}
  ],
  "platforms": {
    "snp": {
      "policy": {"smt": true, "migrate-ma": false, "debug": false}
    },
    "native": {}
  }
}
```

SVSM and OVMF each have their own ACPI tables at different GPAs. Each component
discovers its tables from its own metadata or by convention — the VMM just loads
everything into the flat GPA space.

## Platform privilege layers

All three CC architectures provide intra-guest privilege separation with
equivalent semantics:

| Layer | AMD SEV 3.0  | Intel TDX 1.5      | Arm CCA      |
| ----- | ------------ | ------------------ | ------------ |
| 0     | VMPL0 (SVSM) | L1 VMM (paravisor) | P0 (service) |
| 1     | VMPL1        | L2 VM              | P1           |
| 2     | VMPL2        | L2 VM              | P2           |
| 3     | VMPL3        | L2 VM              | P3           |

Layer 0 is always the privileged service module. Layer 1+ is the guest OS or
firmware.

Layers are not expressed in the manifest's `sections` or `parameters` arrays.
During the load phase, every platform uses a single flat GPA space — the
hardware APIs take an address, not a layer. Layer annotations appear only in the
platform config, where they are meaningful: a VMSA targets a specific VMPL, a
secrets page is accessible at a specific privilege level. The guest's privileged
component (SVSM, paravisor, P0) manages per-layer memory views post-boot.

## Measurement

Measurement is not defined by the manifest. It is defined by the CC platform.

The VMM feeds measured sections to the platform's measurement API in array order
during step 4 of the execution model. Parameters (step 5) are typically
unmeasured. Platform-specific data loaded during steps 3 and 6 is also measured
by the platform as appropriate — but the ordering and measurement rules for
those steps are defined by the platform's binding specification, not by PMI.

- **AMD SEV 3.0:** `SNP_LAUNCH_UPDATE` per measured section (step 4) and per
  platform page (step 6).
- **Intel TDX:** `KVM_TDX_INIT_MEM_REGION` with `MEASURE` flag per measured
  section (step 4) and TD HOBs (step 6).
- **Arm CCA:** `RMI_DATA_CREATE` per measured section (step 4).

The hash algorithm, digest computation, and attestation report format are all
platform-determined. Offline verification tools compute the expected digest
using the platform's known algorithm, the section contents from the PE, and the
platform binding's convention for ordering of steps 3 and 6 relative to step 4
in the measurement stream.

In serviced configurations, the launch measurement covers the service module and
firmware. Kernel boot is measured separately by firmware via the service
module's vTPM into runtime measurement registers (TPM PCRs, Intel TDX RTMRs, Arm
CCA extensible measurements). A verifier needs both the launch digest and the
runtime measurement quotes.
