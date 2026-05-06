# Sections

The manifest's `sections` array is the core of a PMI image. It is an ordered
list of everything the VMM loads or generates when launching a guest. See the
[manifest schema](README.md#schema) for the top-level structure.

## Schema

```cddl
section = {
  "name"         => tstr,               ; PE section name (e.g., ".ovmf", ".sev.svm")
  ? "fill"        => fill,              ; VMM-generated content; absent = load from disk
  ? "platforms"   => { + tstr => any },  ; platform name => platform-defined annotation
  ? "measured"    => bool,              ; default true
  * tstr => any,                        ; extension point
}
```

## Extensibility

Every PMI-defined map accepts additional keys beyond those defined here.
Well-known keys are short, unnamespaced strings (e.g., `"name"`, `"measured"`,
`"sev"`). Extension keys MUST use a collision-resistant namespaced form:
`"namespace:key"` (e.g., `"vendor:feature"`). Well-known fill values use the
same namespaced convention.

Consumers MUST ignore keys and fill values they do not recognize.

## Processing Order

The VMM processes sections in array order during
[step 5](../overview.md#vmm-execution-model) of the execution model. Measurement
follows the same order.

Each section references a PE section by name. The VMM reads `VirtualAddress`,
`SizeOfRawData`, `VirtualSize`, and `PointerToRawData` from the PE section
header.

## Data Sections

When `fill` is absent, the section is a data section. The VMM loads on-disk data
from the PE into guest memory at `VirtualAddress`.

## Filled Sections

When `fill` is present, the corresponding PE section has `SizeOfRawData == 0`
and `VirtualSize > 0` — it reserves an address range with no on-disk data. The
VMM generates content based on the `fill` value and writes it into the region at
`VirtualAddress`. Filled sections SHOULD be unmeasured (`"measured": false`)
since their content is VMM-generated and cannot be predicted by a verifier.

A section MUST NOT have both `fill` and a non-null platform annotation.

The `fill` field is a map with a required `"type"` key that identifies the
fill type. Additional keys are type-specific parameters.

```cddl
fill = {
  "type"         => tstr,              ; fill type identifier
  * tstr => any,                       ; type-specific parameters
}
```

VMMs MUST reject fill types they do not recognize. Well-known fill types use
the `"namespace:type"` convention. The following are defined by PMI:

### `"efi:memmap"` — EFI Memory Map

The VMM MUST generate an EFI memory map describing the guest's physical address
space and write it as an array of `EFI_MEMORY_DESCRIPTOR` structures at the
section's `VirtualAddress`. The map MUST cover all memory regions visible to the
guest, including regions established by prior sections in the manifest.

The number of entries and their types are determined by the VMM based on the
guest's memory layout. The PE section's `VirtualSize` MUST be large enough to
hold the generated map; the VMM MUST NOT write beyond `VirtualSize`.

### `"acpi:rsdp"` — ACPI Root System Description Pointer

The VMM MUST generate an ACPI RSDP (Root System Description Pointer) structure
and write it at the section's `VirtualAddress`. The RSDP points to the RSDT
and/or XSDT, which in turn reference other ACPI tables. The VMM is responsible
for generating a consistent set of ACPI tables that describe the guest's
hardware topology.

### `"acpi:srat"` — ACPI System Resource Affinity Table

The VMM MUST generate an ACPI SRAT describing the guest's NUMA topology and
write it at the section's `VirtualAddress`. Each entry maps a processor or
memory range to a proximity domain. The PE section's `VirtualSize` MUST be large
enough to hold the generated table.

### `"acpi:madt"` — ACPI Multiple APIC Description Table

The VMM MUST generate an ACPI MADT describing the guest's interrupt controller
topology and write it at the section's `VirtualAddress`. The table includes
entries for each local APIC, I/O APIC, and interrupt source override. The PE
section's `VirtualSize` MUST be large enough to hold the generated table.

## Platform Annotations

The `platforms` field, when present, is a map from platform name to a
platform-defined value. If the current platform is not a key in the map, the
section is skipped.

- A `null` value means "load this section on this platform with no special
  behavior."
- A non-null value is interpreted by the platform adapter — for example, SEV 3.0
  uses string values to indicate page types (`"vmsa"`, `"secrets"`, `"cpuid"`).
  See [platforms/sev.md](platforms/sev.md) for details.

Sections with a non-null platform annotation are loaded in step 5 using the
platform adapter's section-specific API, in array order alongside all other
sections.

If `platforms` is absent, the section is loaded on all platforms during step 5.

## Section Loading

For each section loaded in step 5, the VMM reads the PE section header and
determines how to load it based on `VirtualAddress`, `SizeOfRawData`,
`VirtualSize`, and `PointerToRawData`.

The VMM loads pages from the lowest GPA to the highest within each section. This
ordering is significant: CC platforms measure pages in submission order, so
lowest-to-highest produces a deterministic measurement.

There are three cases:

1. **Data section** (`SizeOfRawData > 0`, `VirtualSize == SizeOfRawData`). Load
   the on-disk data at `VirtualAddress`. The VMM chooses page granularity based
   on alignment — see [overview](../pe.md#page-granularity).

2. **Padded section** (`SizeOfRawData > 0`, `VirtualSize > SizeOfRawData`). Load
   the on-disk data at `VirtualAddress` as in case 1. Then zero-fill from
   `VirtualAddress + SizeOfRawData` to `VirtualAddress + VirtualSize`. The
   trailing zero region SHOULD use the platform's zero-page API where available
   (e.g., `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_ZERO`), which validates pages as
   zero without transferring data. This is standard PE .bss-tail behavior —
   firmware or service modules that need reserved memory beyond their code use
   this to express it without file backing.

3. **Zero section** (`SizeOfRawData == 0`, `VirtualSize > 0`). The entire region
   is zero-filled. No disk data is loaded. The VMM SHOULD use the platform's
   zero-page API for the full range. This is how reserved memory regions are
   expressed — for example, SEV secrets pages and CPUID pages that the platform
   adapter populates via their platform annotation.

## Measurement

If `measured` is true (the default), the section's data is fed to the platform's
measurement API during loading.

The distinction between on-disk data and zero-fill matters for measurement.
On-disk bytes are measured as normal data pages. Zero-filled bytes are measured
as zero pages using the platform's zero-page measurement semantic, which may
produce a different measurement than loading actual zeros as data pages. VMM
implementations MUST NOT substitute data-page loads for zero-page operations or
vice versa.

Filled sections SHOULD be unmeasured since their content is VMM-generated.
Platform-annotated sections are measured by the platform as appropriate — the
measurement rules for platform-annotated sections are defined by the platform's
binding specification, not by PMI.

In serviced configurations, the launch measurement covers the service module and
firmware. Kernel boot is measured separately by firmware via the service
module's virtual TPM (vTPM) into runtime measurement registers. A verifier needs
both the launch digest and the runtime measurement quotes — neither alone is
sufficient.
