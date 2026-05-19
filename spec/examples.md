# Examples

A PMI image is a PE binary that, for each supported target, carries a CBOR
spec in a `.pmi.<target>` PE section. A VMM targeting one of them reads its
section and follows the recipe. PE section names other than the target spec
sections themselves are free-form.

## Direct boot on `vm` and `sev`

### `.pmi.vm`

```cbor-diag
{
  "version": 1,
  "dtb": ".dtb.vm",
  "vcpu": {"rip": 0x100000, "rsp": 0x80000, "rflags": 0x2, "cs": {...}, ...},
  "actions": [
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"},
    {"type": "dtbo", "section": ".dtbo"}
  ]
}
```

### `.pmi.sev`

```cbor-diag
{
  "version": 1,
  "dtb": ".dtb.sev",
  "actions": [
    {"type": "load",       "section": ".linux"},
    {"type": "load",       "section": ".initrd"},
    {"type": "load",       "section": ".cmdline"},
    {"type": "dtbo",       "section": ".dtbo"},
    {"type": "sev:vmsa",   "section": ".sev.vms"}
  ]
}
```

**`vm` launch (steps 1–6):**

1. Read `.pmi.vm`.
2. Parse `.dtb.vm`; validate host conformance.
3. (No CC init for `vm`.)
4. Process actions in order: `load` `.linux`, `.initrd`, `.cmdline`
   (measured by default). Write the host-decided memory/cpus/NUMA overlay
   into `.dtbo` (unmeasured).
5. Apply the spec's `vcpu` register map to the boot vCPU.
6. Kernel starts.

**`sev` launch (steps 1–6):**

1. Read `.pmi.sev`.
2. Parse `.dtb.sev`; validate host conformance.
3. `SNP_LAUNCH_START` with the host-supplied launch policy.
4. `SNP_LAUNCH_UPDATE` for `.linux`, `.initrd`, `.cmdline` (measured).
   Write the host overlay into `.dtbo` (unmeasured). `SNP_LAUNCH_UPDATE`
   with `page_type=vmsa` for `.sev.vms`.
5. `SNP_LAUNCH_FINISH` (no id_block in this example).
6. Kernel starts.

**Bare metal:** UEFI executes the EFI stub in `.linux`. All `.pmi.*` and
other non-loaded PE sections are ignored. Standard UKI boot.

## Serviced SEV with signed ID block

### `.pmi.sev`

```cbor-diag
{
  "version": 1,
  "dtb": ".dtb.sev",
  "id-block": ".sev.idb",
  "id-auth": ".sev.ida",
  "actions": [
    {"type": "load",        "section": ".sev.svm"},
    {"type": "load",        "section": ".ovmf"},
    {"type": "load",        "section": ".linux"},
    {"type": "load",        "section": ".initrd"},
    {"type": "load",        "section": ".cmdline"},
    {"type": "load",        "section": ".osrel"},
    {"type": "dtbo",        "section": ".dtbo"},
    {"type": "sev:secrets", "section": ".sev.sec"},
    {"type": "sev:cpuid",   "section": ".sev.cpu"},
    {"type": "sev:vmsa",    "section": ".sev.vms"}
  ]
}
```

**SEV launch:**

1. Read `.pmi.sev`.
2. Parse `.dtb.sev`; validate host conformance.
3. `SNP_LAUNCH_START` with the host-supplied launch policy, verified
   compatible with the policy embedded in the signed `.sev.idb`.
4. `SNP_LAUNCH_UPDATE` for `.sev.svm`, `.ovmf` (measured). Skip
   `.linux`/`.initrd`/`.cmdline`/`.osrel` if doing indirect boot (OVMF
   boots kernel from disk). Write the host overlay into `.dtbo`
   (unmeasured). `SNP_LAUNCH_UPDATE` with the appropriate page types for
   `.sev.sec`, `.sev.cpu`, `.sev.vms`.
5. `SNP_LAUNCH_FINISH` with `id_block` from `.sev.idb` and `id_auth` from
   `.sev.ida`.
6. SVSM starts at VMPL0, initializes vTPM, creates VMPL1 VMSA for OVMF,
   transitions OVMF. OVMF boots kernel from disk, measures boot via SVSM
   vTPM.

**Non-CC VM:** the image does not carry a `.pmi.vm` section; a VMM
targeting `vm` refuses to launch this image.

**Bare metal:** UEFI ignores `.pmi.*` and all `.sev.*`, `.ovmf`, `.dtb.sev`
sections. EFI stub in `.linux` executes normally.

## Both `vm` and SEV serviced boot in one image

A PMI image supporting both `vm` and SEV serviced boot might contain the
following PE sections. Only the `.pmi.<target>` names are used by PMI to
discover target specs; all other names shown are illustrative.

| Section    | Loaded by UEFI? | Purpose                                    |
| ---------- | --------------- | ------------------------------------------ |
| `.linux`   | Yes (via stub)  | Kernel                                     |
| `.initrd`  | Yes (via stub)  | Initial ramdisk                            |
| `.cmdline` | Yes (via stub)  | Kernel command line                        |
| `.dtb.vm`  | No              | Base DTB used by the `vm` spec             |
| `.dtb.sev` | No              | Base DTB used by the `sev` spec            |
| `.dtbo`    | No              | Host-filled DTB overlay (memory/cpus/numa) |
| `.ovmf`    | No              | Guest firmware                             |
| `.sev.svm` | No              | SVSM service module                        |
| `.sev.vms` | No              | SEV VMSA register state                    |
| `.sev.sec` | No              | SEV secrets page                           |
| `.sev.cpu` | No              | SEV CPUID page                             |
| `.sev.idb` | No              | SEV ID block                               |
| `.sev.ida` | No              | SEV ID auth info                           |
| `.pmi.vm`  | No              | `vm` target spec                           |
| `.pmi.sev` | No              | `sev` target spec                          |

On bare metal, UEFI executes the EFI stub, which boots the kernel from
`.linux`. All `.pmi.*` and other non-loaded PE sections are ignored.

A VMM targeting `vm` reads `.pmi.vm`, inspects its `dtb`, validates
conformance, processes its actions, and starts the guest.

A VMM targeting `sev` reads `.pmi.sev`. Its actions drive the SEV-SNP
launch APIs (`SNP_LAUNCH_START`, `SNP_LAUNCH_UPDATE`,
`SNP_LAUNCH_FINISH`), with the launch digest covering everything fed to
the target's measurement API.

One artifact. One spec per supported target. The image carries exactly
the launch paths it advertises and nothing more.
