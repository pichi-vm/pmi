# Examples

## Direct Boot with CC (AMD SEV 3.0)

```cbor-diag
{
  "version": 1,
  "metadata": [
    {"name": ".dtb", "type": "dtb"}
  ],
  "segments": [
    {"name": ".linux"},
    {"name": ".initrd"},
    {"name": ".cmdline"},
    {"name": ".dtbo",   "fill": {"type": "pmi:dtbo"}},
    {"name": ".sev.vms", "platforms": {"sev": "vmsa"}}
  ],
  "policy": {
    "sev": {"smt": true, "migrate-ma": false, "debug": false},
    "native": {}
  }
}
```

**SEV 3.0 (steps 1–9):**

1. Select `"sev"`.
2. Inspect metadata: parse `.dtb` to learn the image's expected platform
   topology and address layout. Validate the host can conform; fail launch
   otherwise.
3. Merge image policy with deployer policy (image wins on conflict).
4. `SNP_LAUNCH_START` with merged policy.
5. (No pre-load for SEV.)
6. Process segments in order: `SNP_LAUNCH_UPDATE` for `.linux`, `.initrd`,
   `.cmdline` (all measured, normal page type). Fill `.dtbo` with the
   host-decided memory/cpus/NUMA overlay (unmeasured). Load `.sev.vms`
   via `SNP_LAUNCH_UPDATE` with `page_type=vmsa` (platform annotation).
7. (No post-load for SEV.)
8. `SNP_LAUNCH_FINISH`.
9. Kernel starts.

**Native:** Steps 3–5, 7, 8 are no-ops. VMM inspects `.dtb`, validates
conformance, loads data segments, fills `.dtbo`, sets registers per the
image's `vcpu` annotation, starts guest.

**Bare metal:** UEFI ignores `.pmi`, `.dtb`, `.dtbo`. EFI stub in `.linux`
executes normally. Standard UKI boot.

## Serviced: SVSM + OVMF (AMD SEV 3.0)

```cbor-diag
{
  "version": 1,
  "metadata": [
    {"name": ".dtb", "type": "dtb"}
  ],
  "segments": [
    {"name": ".sev.svm", "platforms": {"sev": null}},
    {"name": ".ovmf"},
    {"name": ".linux"},
    {"name": ".initrd"},
    {"name": ".cmdline"},
    {"name": ".osrel"},
    {"name": ".dtbo",   "fill": {"type": "pmi:dtbo"}},
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

**SEV 3.0 (steps 1–9):**

1. Select `"sev"`.
2. Inspect `.dtb` metadata; validate host conformance.
3. Merge image policy with deployer policy (image wins on conflict).
4. `SNP_LAUNCH_START` with merged policy.
5. (No pre-load for SEV.)
6. Process segments in order: `SNP_LAUNCH_UPDATE` for `.sev.svm`, `.ovmf`
   (all measured, normal page type). Skip
   `.linux`/`.initrd`/`.cmdline`/`.osrel` if doing indirect boot (OVMF
   boots kernel from disk). Fill `.dtbo` with the host-decided
   memory/cpus/NUMA overlay (unmeasured). Load `.sev.sec`
   (`page_type=secrets`), `.sev.cpu` (`page_type=cpuid`), `.sev.vms`
   (`page_type=vmsa`) via `SNP_LAUNCH_UPDATE` with the corresponding
   page types (platform annotations).
7. (No post-load for SEV.)
8. `SNP_LAUNCH_FINISH`.
9. SVSM starts at VMPL0, initializes vTPM, creates VMPL1 VMSA for OVMF,
   transitions OVMF. OVMF boots kernel from disk, measures boot via SVSM
   vTPM.

**Native (steps 1–9):**

1. Select `"native"`.
2. Inspect `.dtb`; validate conformance.
3. Merge policy (native has no policy fields — no-op).
4. (No-op.)
5. (No-op.)
6. Skip `.sev.svm`, `.sev.sec`, `.sev.cpu`, `.sev.vms` segments (filtered — not
   in `"native"`). Load `.ovmf`, `.linux`, `.initrd`, `.cmdline`, `.osrel`.
   Fill `.dtbo`.
7. (No-op.)
8. (No-op.)
9. Set registers from `vcpu` annotation. OVMF boots kernel from disk.

**Bare metal:** UEFI ignores `.pmi`, `.dtb`, `.dtbo`, `.sev.svm`, `.ovmf`
(non-loaded PE sections). EFI stub in `.linux` executes normally. Standard
UKI boot.

One artifact. One manifest. Three execution paths.

## Per-Platform Base DTBs

```cbor-diag
{
  "version": 1,
  "metadata": [
    {"name": ".dtb.native", "type": "dtb", "platforms": {"native": null}},
    {"name": ".dtb.sev",    "type": "dtb", "platforms": {"sev":    null}}
  ],
  "segments": [
    {"name": ".sev.svm", "platforms": {"sev": null}},
    {"name": ".ovmf"},
    {"name": ".linux"},
    {"name": ".initrd"},
    {"name": ".cmdline"},
    {"name": ".dtbo", "fill": {"type": "pmi:dtbo"}}
  ],
  "policy": {
    "sev": {"smt": true, "migrate-ma": false, "debug": false},
    "native": {}
  }
}
```

The image carries two base DTBs: one for `native` (plain virtual
platform), one for `sev` (with SEV-specific platform topology, e.g.
SVSM-provided vTPM nodes). The VMM picks the matching one based on the
current platform's `metadata` annotation. The host overlay (`.dtbo`) is
the same regardless of platform — resource info is platform-agnostic.

If both base DTBs are at the same `VirtualAddress` (per the
[VirtualAddress sharing rule](pe.md#virtualaddress-sharing-for-mutually-exclusive-sections)),
the image's stub finds the base at a single fixed GPA without needing
platform awareness at runtime.
