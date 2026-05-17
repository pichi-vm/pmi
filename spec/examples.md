# Examples

## Direct Boot with CC (AMD SEV 3.0)

```cbor-diag
{
  "version": 1,
  "dtb": [
    {"section": ".dtb"}
  ],
  "segments": [
    {"section": ".linux"},
    {"section": ".initrd"},
    {"section": ".cmdline"},
    {"section": ".dtbo", "type": "pmi:dtbo"},
    {"section": ".sev.vms", "type": "pmi:sev:vmsa", "platforms": ["sev"]},
    {"section": ".vcpu", "type": "pmi:native:vcpu", "platforms": ["native"]}
  ],
  "policy": {
    "sev": {"smt": true, "migrate-ma": false, "debug": false},
    "native": {}
  }
}
```

**SEV 3.0 (steps 1–9):**

1. Select `"sev"`.
2. Parse the `.dtb` FDT to learn the image's expected platform topology and
   address layout. Validate the host can conform; fail launch otherwise.
3. Merge image policy with deployer policy (image wins on conflict).
4. `SNP_LAUNCH_START` with merged policy.
5. (No pre-load for SEV.)
6. Process segments in order: `SNP_LAUNCH_UPDATE` for `.linux`, `.initrd`,
   `.cmdline` (all `pmi:load`, measured, normal page type). Write the
   host-decided memory/cpus/NUMA overlay into `.dtbo` (`pmi:dtbo`, unmeasured).
   Load `.sev.vms` via `SNP_LAUNCH_UPDATE` with `page_type=vmsa`
   (`pmi:sev:vmsa`). Skip the `.vcpu` segment (filtered — not in `"sev"`).
7. (No post-load for SEV.)
8. `SNP_LAUNCH_FINISH`.
9. Kernel starts.

**Native:** Steps 3–5, 7, 8 are no-ops. VMM parses `.dtb`, validates
conformance, loads `pmi:load` segments, writes `.dtbo`, sets registers from
the `pmi:native:vcpu` segment, starts guest. The `.sev.vms` segment is skipped
(filtered — not in `"native"`).

**Bare metal:** UEFI ignores `.pmi`, `.dtb`, `.dtbo`. EFI stub in `.linux`
executes normally. Standard UKI boot.

## Serviced: SVSM + OVMF (AMD SEV 3.0)

```cbor-diag
{
  "version": 1,
  "dtb": [
    {"section": ".dtb"}
  ],
  "segments": [
    {"section": ".sev.svm", "platforms": ["sev"]},
    {"section": ".ovmf"},
    {"section": ".linux"},
    {"section": ".initrd"},
    {"section": ".cmdline"},
    {"section": ".osrel"},
    {"section": ".dtbo",    "type": "pmi:dtbo"},
    {"section": ".sev.sec", "type": "pmi:sev:secrets", "platforms": ["sev"]},
    {"section": ".sev.cpu", "type": "pmi:sev:cpuid",   "platforms": ["sev"]},
    {"section": ".sev.vms", "type": "pmi:sev:vmsa",    "platforms": ["sev"]},
    {"section": ".vcpu",    "type": "pmi:native:vcpu", "platforms": ["native"]}
  ],
  "policy": {
    "sev": {"smt": true, "migrate-ma": false, "debug": false},
    "native": {}
  }
}
```

**SEV 3.0 (steps 1–9):**

1. Select `"sev"`.
2. Parse the `.dtb` FDT; validate host conformance.
3. Merge image policy with deployer policy (image wins on conflict).
4. `SNP_LAUNCH_START` with merged policy.
5. (No pre-load for SEV.)
6. Process segments in order: `SNP_LAUNCH_UPDATE` for `.sev.svm`, `.ovmf`
   (all `pmi:load`, measured, normal page type). Skip
   `.linux`/`.initrd`/`.cmdline`/`.osrel` if doing indirect boot (OVMF
   boots kernel from disk). Write the host-decided memory/cpus/NUMA overlay
   into `.dtbo` (`pmi:dtbo`, unmeasured). Load `.sev.sec`
   (`pmi:sev:secrets`), `.sev.cpu` (`pmi:sev:cpuid`), `.sev.vms`
   (`pmi:sev:vmsa`) via `SNP_LAUNCH_UPDATE` with the corresponding page
   types. Skip `.vcpu` (filtered — not in `"sev"`).
7. (No post-load for SEV.)
8. `SNP_LAUNCH_FINISH`.
9. SVSM starts at VMPL0, initializes vTPM, creates VMPL1 VMSA for OVMF,
   transitions OVMF. OVMF boots kernel from disk, measures boot via SVSM
   vTPM.

**Native (steps 1–9):**

1. Select `"native"`.
2. Parse the `.dtb` FDT; validate conformance.
3. Merge policy (native has no policy fields — no-op).
4. (No-op.)
5. (No-op.)
6. Skip `.sev.svm`, `.sev.sec`, `.sev.cpu`, `.sev.vms` segments (filtered — not
   in `"native"`). Load `.ovmf`, `.linux`, `.initrd`, `.cmdline`, `.osrel`.
   Write `.dtbo`.
7. (No-op.)
8. (No-op.)
9. Set registers from the `pmi:native:vcpu` segment. OVMF boots kernel from
   disk.

**Bare metal:** UEFI ignores `.pmi`, `.dtb`, `.dtbo`, `.sev.svm`, `.ovmf`,
`.vcpu` (non-loaded PE sections). EFI stub in `.linux` executes normally.
Standard UKI boot.

One artifact. One manifest. Three execution paths.

## Per-Platform Base DTBs

```cbor-diag
{
  "version": 1,
  "dtb": [
    {"section": ".dtb.sev",    "platforms": ["sev"]},
    {"section": ".dtb.native", "platforms": ["native"]}
  ],
  "segments": [
    {"section": ".sev.svm", "platforms": ["sev"]},
    {"section": ".ovmf"},
    {"section": ".linux"},
    {"section": ".initrd"},
    {"section": ".cmdline"},
    {"section": ".dtbo", "type": "pmi:dtbo"},
    {"section": ".vcpu", "type": "pmi:native:vcpu", "platforms": ["native"]}
  ],
  "policy": {
    "sev": {"smt": true, "migrate-ma": false, "debug": false},
    "native": {}
  }
}
```

The image carries two base DTBs: one for `sev` (with SEV-specific platform
topology, e.g. SVSM-provided vTPM nodes), one for `native` (plain virtual
platform). The VMM picks the first `dtb` entry whose `platforms` filter
matches the current platform (see [DTB selection](manifest/dtb.md#selection)).
The host overlay (`.dtbo`) is the same regardless of platform — resource info
is platform-agnostic.

If both base DTBs are at the same `VirtualAddress` (per the
[VirtualAddress sharing rule](pe.md#virtualaddress-sharing-for-mutually-exclusive-sections)),
the image's stub finds the base at a single fixed GPA without needing
platform awareness at runtime.
