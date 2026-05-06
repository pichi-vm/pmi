# Examples

## Direct Boot with CC (AMD SEV 3.0)

```cbor-diag
{
  "version": 1,
  "sections": [
    {"name": ".linux"},
    {"name": ".initrd"},
    {"name": ".cmdline"},
    {"name": ".acpi"},
    {"name": ".bprms"},
    {"name": ".memmap",  "fill": {"type": "efi:memmap"}},
    {"name": ".sev.vms", "platforms": {"sev": "vmsa"}}
  ],
  "policy": {
    "sev": {"smt": true, "migrate-ma": false, "debug": false},
    "native": {}
  }
}
```

**SEV 3.0 (steps 1–8):**

1. Select `"sev"`.
2. Merge image policy with deployer policy (image wins on conflict).
3. `SNP_LAUNCH_START` with merged policy.
4. (No pre-load for SEV.)
5. Process sections in order: `SNP_LAUNCH_UPDATE` for `.linux`, `.initrd`,
   `.cmdline`, `.acpi`, `.bprms` (all measured, normal page type). Fill
   `.memmap` with EFI memory map (unmeasured). Load `.sev.vms` via
   `SNP_LAUNCH_UPDATE` with `page_type=vmsa` (platform annotation).
6. (No post-load for SEV.)
7. `SNP_LAUNCH_FINISH`.
8. Kernel starts.

**Native:** Steps 2–4, 6, 7 are no-ops. VMM loads data sections, fills
`.memmap`, sets registers to kernel entry, starts guest.

**Bare metal:** UEFI ignores `.pmi`. EFI stub in `.linux` executes normally.
Standard UKI boot.

## Serviced: SVSM + OVMF (AMD SEV 3.0)

```cbor-diag
{
  "version": 1,
  "sections": [
    {"name": ".sev.svm", "platforms": {"sev": null}},
    {"name": ".ovmf"},
    {"name": ".linux"},
    {"name": ".initrd"},
    {"name": ".cmdline"},
    {"name": ".osrel"},
    {"name": ".memmap",  "fill": {"type": "efi:memmap"}},
    {"name": ".sev.sec", "platforms": {"sev": "secrets"}},
    {"name": ".sev.cpu", "platforms": {"sev": "cpuid"}},
    {"name": ".sev.vms", "platforms": {"sev": "vmsa"}}
  ],
  "policy": {
    "sev": {"smt": true, "migrate-ma": false, "debug": false},
    "native": {}
  }
}
```

**SEV 3.0 (steps 1–8):**

1. Select `"sev"`.
2. Merge image policy with deployer policy (image wins on conflict).
3. `SNP_LAUNCH_START` with merged policy.
4. (No pre-load for SEV.)
5. Process sections in order: `SNP_LAUNCH_UPDATE` for `.sev.svm`, `.ovmf` (all
   measured, normal page type). Skip `.linux`/`.initrd`/`.cmdline`/`.osrel` if
   doing indirect boot (OVMF boots kernel from disk). Fill `.memmap` with EFI
   memory map (unmeasured). Load `.sev.sec` (`page_type=secrets`), `.sev.cpu`
   (`page_type=cpuid`), `.sev.vms` (`page_type=vmsa`) via `SNP_LAUNCH_UPDATE`
   with the corresponding page types (platform annotations).
6. (No post-load for SEV.)
7. `SNP_LAUNCH_FINISH`.
8. SVSM starts at VMPL0, initializes vTPM, creates VMPL1 VMSA for OVMF,
   transitions OVMF. OVMF boots kernel from disk, measures boot via SVSM vTPM.

**Native (steps 1–8):**

1. Select `"native"`.
2. Merge policy (native has no policy fields — no-op).
3. (No-op.)
4. (No-op.)
5. Skip `.sev.svm`, `.sev.sec`, `.sev.cpu`, `.sev.vms` (filtered — not in
   `"native"`). Load `.ovmf`, `.linux`, `.initrd`, `.cmdline`, `.osrel`. Fill
   `.memmap`.
6. (No-op.)
7. (No-op.)
8. Set registers to OVMF reset vector. OVMF boots kernel from disk.

**Bare metal:** UEFI ignores `.pmi`, `.sev.svm`, `.ovmf` (non-loaded sections).
EFI stub in `.linux` executes normally. Standard UKI boot.

One artifact. One manifest. Three execution paths.

## Per-Component ACPI

```cbor-diag
{
  "version": 1,
  "sections": [
    {"name": ".sev.svm", "platforms": {"sev": null}},
    {"name": ".acpi0",   "platforms": {"sev": null}},
    {"name": ".acpi1"},
    {"name": ".ovmf"},
    {"name": ".linux"},
    {"name": ".initrd"},
    {"name": ".cmdline"}
  ],
  "policy": {
    "sev": {"smt": true, "migrate-ma": false, "debug": false},
    "native": {}
  }
}
```

SVSM and OVMF each have their own ACPI tables at different GPAs. Each component
discovers its tables from its own metadata or by convention — the VMM just loads
everything into the flat GPA space.
