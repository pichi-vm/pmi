# Examples

A PMI image is a PE binary that, for each supported platform, carries a CBOR
spec in a `.pmi.<plat>` PE section. A VMM targeting a platform reads its
section and follows the recipe. PE section names other than the platform
spec sections themselves are free-form.

## Direct boot on `vm` and `sev`

### `.pmi.vm`

```cbor-diag
{
  "version": 1,
  "dtb": ".dtb.vm",
  "actions": [
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"},
    {"type": "dtbo", "section": ".dtbo"},
    {"type": "vcpu", "section": ".vcpu"}
  ]
}
```

### `.pmi.sev`

```cbor-diag
{
  "version": 1,
  "dtb": ".dtb.sev",
  "actions": [
    {"type": "sev:policy", "section": ".sev.pol"},
    {"type": "load",       "section": ".linux"},
    {"type": "load",       "section": ".initrd"},
    {"type": "load",       "section": ".cmdline"},
    {"type": "dtbo",       "section": ".dtbo"},
    {"type": "sev:vmsa",   "section": ".sev.vms"}
  ]
}
```

**`vm` launch (steps 1–9):**

1. Read `.pmi.vm`.
2. Parse `.dtb.vm`; validate host conformance.
3. _(reserved)_
4. (No CC init for `vm`.)
5. _(reserved)_
6. Process actions in order: `load` `.linux`, `.initrd`, `.cmdline`
   (measured by default). Write the host-decided memory/cpus/NUMA overlay
   into `.dtbo` (unmeasured). Set boot vCPU registers from `.vcpu`.
7. _(reserved)_
8. (No finalize.)
9. Kernel starts.

**`sev` launch (steps 1–9):**

1. Read `.pmi.sev`.
2. Parse `.dtb.sev`; validate host conformance.
3. _(reserved)_
4. `SNP_LAUNCH_START` with the policy from `.sev.pol`.
5. _(reserved)_
6. `SNP_LAUNCH_UPDATE` for `.linux`, `.initrd`, `.cmdline` (measured).
   Write the host overlay into `.dtbo` (unmeasured). `SNP_LAUNCH_UPDATE`
   with `page_type=vmsa` for `.sev.vms`.
7. _(reserved)_
8. `SNP_LAUNCH_FINISH` (no id_block in this example).
9. Kernel starts.

**Bare metal:** UEFI executes the EFI stub in `.linux`. All `.pmi.*` and
other non-loaded PE sections are ignored. Standard UKI boot.

## Serviced SEV with signed ID block

### `.pmi.sev`

```cbor-diag
{
  "version": 1,
  "dtb": ".dtb.sev",
  "actions": [
    {"type": "load",         "section": ".sev.svm"},
    {"type": "load",         "section": ".ovmf"},
    {"type": "load",         "section": ".linux"},
    {"type": "load",         "section": ".initrd"},
    {"type": "load",         "section": ".cmdline"},
    {"type": "load",         "section": ".osrel"},
    {"type": "dtbo",         "section": ".dtbo"},
    {"type": "sev:secrets",  "section": ".sev.sec"},
    {"type": "sev:cpuid",    "section": ".sev.cpu"},
    {"type": "sev:vmsa",     "section": ".sev.vms"},
    {"type": "sev:id-block", "section": ".sev.idb"},
    {"type": "sev:id-auth",  "section": ".sev.ida"}
  ]
}
```

**SEV launch:**

1. Read `.pmi.sev`.
2. Parse `.dtb.sev`; validate host conformance.
3. _(reserved)_
4. Extract policy from the signed `.sev.idb`. `SNP_LAUNCH_START` with that
   policy.
5. _(reserved)_
6. `SNP_LAUNCH_UPDATE` for `.sev.svm`, `.ovmf` (measured). Skip
   `.linux`/`.initrd`/`.cmdline`/`.osrel` if doing indirect boot (OVMF
   boots kernel from disk). Write the host overlay into `.dtbo`
   (unmeasured). `SNP_LAUNCH_UPDATE` with the appropriate page types for
   `.sev.sec`, `.sev.cpu`, `.sev.vms`.
7. _(reserved)_
8. `SNP_LAUNCH_FINISH` with `id_block` from `.sev.idb` and `id_auth` from
   `.sev.ida`.
9. SVSM starts at VMPL0, initializes vTPM, creates VMPL1 VMSA for OVMF,
   transitions OVMF. OVMF boots kernel from disk, measures boot via SVSM
   vTPM.

**Non-CC VM:** the image does not carry a `.pmi.vm` section; a VMM
targeting `vm` refuses to launch this image.

**Bare metal:** UEFI ignores `.pmi.*` and all `.sev.*`, `.ovmf`, `.dtb.sev`
sections. EFI stub in `.linux` executes normally.

One artifact. One spec per supported platform. The image carries exactly
the launch paths it advertises and nothing more.
