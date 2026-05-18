# Examples

A PMI image is a PE binary with three logical layers:

1. The [PMI index](index.md) in `.pmi` lists supported platforms and points
   at each one's per-platform manifest section.
2. Each per-platform [manifest](manifest/README.md) (by convention
   `.pmi.<plat>`) is a complete launch recipe for that platform.
3. Per-platform manifests reference other PE sections by name (kernel, initrd,
   firmware, DTBs, platform-specific pages, etc.).

`.pmi` is the only PE section name PMI requires. Every other PE section name
in the examples below is illustrative — image authors choose names freely.

## Direct boot on `vm` and SEV

### `.pmi` (index)

```cbor-diag
{
  "version": 1,
  "platforms": {
    "vm":  ".pmi.vm",
    "sev": ".pmi.sev"
  }
}
```

### `.pmi.vm` (vm manifest)

```cbor-diag
{
  "version": 1,
  "dtb": ".dtb.vm",
  "segments": [
    {"section": ".linux"},
    {"section": ".initrd"},
    {"section": ".cmdline"},
    {"section": ".dtbo", "type": "pmi:dtbo"},
    {"section": ".vcpu", "type": "pmi:vm:vcpu"}
  ]
}
```

### `.pmi.sev` (SEV manifest)

```cbor-diag
{
  "version": 1,
  "dtb": ".dtb.sev",
  "segments": [
    {"section": ".sev.pol", "type": "pmi:sev:policy"},
    {"section": ".linux"},
    {"section": ".initrd"},
    {"section": ".cmdline"},
    {"section": ".dtbo",    "type": "pmi:dtbo"},
    {"section": ".sev.vms", "type": "pmi:sev:vmsa"}
  ]
}
```

**vm launch (steps 1–9):**

1. Look up `"vm"` in `.pmi` → read `.pmi.vm`.
2. Parse `.dtb.vm`; validate host conformance.
3. _(reserved)_
4. (No CC init for `vm`.)
5. (No pre-load.)
6. Process segments in order: load `.linux`, `.initrd`, `.cmdline` (default
   `pmi:load`, measured). Write the host-decided memory/cpus/NUMA overlay into
   `.dtbo` (`pmi:dtbo`, unmeasured). Set boot vCPU registers from the
   `pmi:vm:vcpu` segment.
7. (No post-load.)
8. (No finalize.)
9. Kernel starts.

**SEV launch (steps 1–9):**

1. Look up `"sev"` in `.pmi` → read `.pmi.sev`.
2. Parse `.dtb.sev`; validate host conformance.
3. _(reserved)_
4. `SNP_LAUNCH_START` with the policy from `.sev.pol` (`pmi:sev:policy`).
5. (No pre-load.)
6. `SNP_LAUNCH_UPDATE` for `.linux`, `.initrd`, `.cmdline` (default
   `pmi:load`, measured). Write the host-decided overlay into `.dtbo`
   (`pmi:dtbo`, unmeasured). `SNP_LAUNCH_UPDATE` with `page_type=vmsa` for
   `.sev.vms` (`pmi:sev:vmsa`).
7. (No post-load.)
8. `SNP_LAUNCH_FINISH` (no id_block in this example).
9. Kernel starts.

**Bare metal:** UEFI executes the EFI stub in `.linux`. All `.pmi*` and other
non-loaded PE sections are ignored. Standard UKI boot.

## Serviced SEV with signed ID block

### `.pmi` (index)

```cbor-diag
{
  "version": 1,
  "platforms": {
    "sev": ".pmi.sev"
  }
}
```

### `.pmi.sev`

```cbor-diag
{
  "version": 1,
  "dtb": ".dtb.sev",
  "segments": [
    {"section": ".sev.svm"},
    {"section": ".ovmf"},
    {"section": ".linux"},
    {"section": ".initrd"},
    {"section": ".cmdline"},
    {"section": ".osrel"},
    {"section": ".dtbo",    "type": "pmi:dtbo"},
    {"section": ".sev.sec", "type": "pmi:sev:secrets"},
    {"section": ".sev.cpu", "type": "pmi:sev:cpuid"},
    {"section": ".sev.vms", "type": "pmi:sev:vmsa"},
    {"section": ".sev.idb", "type": "pmi:sev:id-block"},
    {"section": ".sev.ida", "type": "pmi:sev:id-auth"}
  ]
}
```

**SEV launch:**

1. Look up `"sev"` in `.pmi` → read `.pmi.sev`.
2. Parse `.dtb.sev`; validate host conformance.
3. _(reserved)_
4. Extract policy from the signed `.sev.idb` (`pmi:sev:id-block`).
   `SNP_LAUNCH_START` with that policy.
5. (No pre-load.)
6. `SNP_LAUNCH_UPDATE` for `.sev.svm`, `.ovmf` (measured, normal page type).
   Skip `.linux`/`.initrd`/`.cmdline`/`.osrel` if doing indirect boot (OVMF
   boots kernel from disk). Write the host overlay into `.dtbo`
   (`pmi:dtbo`, unmeasured). `SNP_LAUNCH_UPDATE` with the corresponding page
   types for `.sev.sec` (`pmi:sev:secrets`), `.sev.cpu` (`pmi:sev:cpuid`),
   `.sev.vms` (`pmi:sev:vmsa`).
7. (No post-load.)
8. `SNP_LAUNCH_FINISH` with `id_block` from `.sev.idb` and `id_auth` from
   `.sev.ida`.
9. SVSM starts at VMPL0, initializes vTPM, creates VMPL1 VMSA for OVMF,
   transitions OVMF. OVMF boots kernel from disk, measures boot via SVSM vTPM.

**Non-CC VM:** the image does not include `"vm"` in its index; a VMM
targeting `vm` refuses to launch this image.

**Bare metal:** UEFI ignores `.pmi*` and all `.sev.*`, `.ovmf`, `.dtb.sev`
sections. EFI stub in `.linux` executes normally.

One artifact. One index. N per-platform manifests. As many execution paths
as the index advertises.
