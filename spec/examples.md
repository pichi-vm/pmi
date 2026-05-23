# Examples

This document is non-normative; the blocks below illustrate the per-target
bindings but are not reference test vectors. The normative wire format and
constraints are defined in the per-target chapters.

A PMI image is a PE binary that, for each supported target, carries a CBOR
spec in a `.pmi.<target>` PE section. A VMM targeting one of them reads its
section and follows the recipe. PE section names other than the target spec
sections themselves are free-form.

## Direct boot on `vm` and `sev`

### `.pmi.vm`

```cbor-diag
{
  "version": 1,
  "vcpu": {"rip": 0x100000, "rsp": 0x80000, "rflags": 0x2},
  "actions": [
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"}
  ]
}
```

(The omitted `vcpu` keys default to zero per `vcpu-x64`; a real boot image
would set `cs`, `cr0`, `cr3`, `cr4`, `efer`, `gdtr`, and `idtr` to values
matching its early entry-point code. See [`vcpu-x64`](vm.md#vcpu-x64)
for the full schema.)

### `.pmi.sev`

```cbor-diag
{
  "version": 1,
  "actions": [
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"},
    {"type": "load", "section": ".sev.vms", "kind": "vmsa"}
  ]
}
```

**`vm` launch (steps 1–5):**

1. Read `.pmi.vm`.
2. (No CC init for `vm`.)
3. Process actions in order: `load` `.linux`, `.initrd`, `.cmdline`
   (vm's only load kind is `unmeasured`, the default).
4. Apply the spec's `vcpu` register map to the boot vCPU.
5. Kernel starts.

**`sev` launch (steps 1–5):**

1. Read `.pmi.sev`.
2. `SNP_LAUNCH_START` with the host-supplied launch policy.
3. Process actions: `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_NORMAL` for
   `.linux`, `.initrd`, `.cmdline` (default kind `measured`).
   `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_VMSA` for `.sev.vms` (kind
   `vmsa`).
4. `SNP_LAUNCH_FINISH` (no `id` in this example).
5. Kernel starts.

**Bare metal:** UEFI executes the EFI stub in `.linux`. All `.pmi.*` and
other non-loaded PE sections are ignored. Standard UKI boot.

An upper layer (e.g., dillo) that needs a base DTB, host-filled DT
overlay, or other platform metadata layers them on via the
[Extensions](extensions.md) namespace (additional PE
sections, additional CBOR keys prefixed `dillo:`, additional fill
kinds prefixed `dillo:`). PMI itself does not carry any of those.

## Serviced SEV with signed ID block

### `.pmi.sev`

```cbor-diag
{
  "version": 1,
  "id": {"block": ".sev.idb", "auth": ".sev.ida"},
  "actions": [
    {"type": "load", "section": ".sev.svm"},
    {"type": "load", "section": ".ovmf"},
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".initrd"},
    {"type": "load", "section": ".cmdline"},
    {"type": "load", "section": ".osrel"},
    {"type": "fill", "section": ".sev.sec", "kind": "secrets"},
    {"type": "fill", "section": ".sev.cpu", "kind": "cpuid"},
    {"type": "load", "section": ".sev.vms", "kind": "vmsa"}
  ]
}
```

**SEV launch:**

1. Read `.pmi.sev`.
2. `SNP_LAUNCH_START` with the host-supplied launch policy, verified
   compatible with the policy embedded in the signed `.sev.idb`.
3. Process actions. `SNP_LAUNCH_UPDATE` with `PAGE_TYPE_NORMAL` for
   `.sev.svm`, `.ovmf` (default load kind `measured`). Skip
   `.linux`/`.initrd`/`.cmdline`/`.osrel` if doing indirect boot (OVMF
   boots kernel from disk). `SNP_LAUNCH_UPDATE` with
   `PAGE_TYPE_SECRETS` / `PAGE_TYPE_CPUID` / `PAGE_TYPE_VMSA` for
   `.sev.sec` / `.sev.cpu` / `.sev.vms`.
4. `SNP_LAUNCH_FINISH` with `id_block` from `.sev.idb` and `id_auth` from
   `.sev.ida`.
5. SVSM starts at VMPL0, initializes vTPM, creates VMPL1 VMSA for OVMF,
   transitions OVMF. OVMF boots kernel from disk, measures boot via SVSM
   vTPM.

**Non-CC VM:** the image does not carry a `.pmi.vm` section; a VMM
targeting `vm` refuses to launch this image.

**Bare metal:** UEFI ignores `.pmi.*` and all `.sev.*`, `.ovmf` sections.
EFI stub in `.linux` executes normally.

## Both `vm` and SEV serviced boot in one image

A PMI image supporting both `vm` and SEV serviced boot might contain the
following PE sections. Only the `.pmi.<target>` names are used by PMI to
discover target specs; all other names shown are illustrative.

| Section    | Loaded by UEFI? | Purpose                                    |
| ---------- | --------------- | ------------------------------------------ |
| `.linux`   | Yes (via stub)  | Kernel                                     |
| `.initrd`  | Yes (via stub)  | Initial ramdisk                            |
| `.cmdline` | Yes (via stub)  | Kernel command line                        |
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

A VMM targeting `vm` reads `.pmi.vm`, processes its actions, and starts
the guest.

A VMM targeting `sev` reads `.pmi.sev`. Its actions drive the SEV-SNP
launch APIs (`SNP_LAUNCH_START`, `SNP_LAUNCH_UPDATE`,
`SNP_LAUNCH_FINISH`), with the launch digest covering everything fed to
the target's measurement API.

One artifact. One spec per supported target. The image carries exactly
the launch paths it advertises and nothing more.
