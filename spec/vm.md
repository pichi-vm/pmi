# `vm` Extension

**Prefix:** `vm`.

The `vm` extension provides the essential functionality for launching a PMI as a
traditional virtual machine. It defines two extension points:

1. The new target [`.pmi.vm`](#1-new-target-pmivm).
2. The new target attribute [`vm:vcpu`](#2-new-target-attribute-vmvcpu).

## 1. New target: `.pmi.vm`

The `.pmi.vm` PE section carries the `vm` target spec, subject to the
[core PE constraints](constraints.md#pe-constraints).

### Launch model

The VMM executes the launch in five ordered steps:

1. Read the `.pmi.vm` PE section.
2. Initialize hypervisor state.
3. Process each entry in `actions` in array order.
4. Initialize the boot vCPU from [`vm:vcpu`](#2-new-target-attribute-vmvcpu).
5. Start the guest.

### Keys

The `.pmi.vm` CBOR map follows the [core target shape](core.md#shape). Its
`version` MUST be `1`. It adds one required key:

- **`vm:vcpu`** — boot-vCPU register map (see
  [§2](#2-new-target-attribute-vmvcpu)). The variant ([`vcpu-x64`](#vcpu-x64) or
  [`vcpu-aarch64`](#vcpu-aarch64)) MUST match `PE.FileHeader.Machine`.

### Validation

The [core validation rules](core.md#validation) apply. In addition, the VMM MUST
refuse to launch on any of:

- `PE.FileHeader.Machine` is unsupported;
- the `vm:vcpu` variant does not match `PE.FileHeader.Machine` (the spec carries
  a `vcpu-x64` map under `0xAA64`, or a `vcpu-aarch64` map under `0x8664`).

### `load`

On `vm`, the [`default`](core.md#kind) kind places the section's bytes in guest
memory per [section shape](core.md#section-shapes); no measurement is performed.
Implementations MAY copy or map the contents into guest memory.

## 2. New target attribute: `vm:vcpu`

`vm:vcpu` is a CBOR map of boot-vCPU register values applied at launch step 4.
The schema is selected by `PE.FileHeader.Machine`: [`vcpu-x64`](#vcpu-x64) for
`0x8664`, [`vcpu-aarch64`](#vcpu-aarch64) for `0xAA64`.

Missing keys default to zero except where noted. The VMM MUST reject any unknown
key. The VMM MUST reject any value exceeding the field width defined by the
architecture schema.

### `vcpu-x64`

```cddl
vcpu-x64 = {
  ? "rip"    => uint,                     ; u64
  ? "rsp"    => uint,                     ; u64
  ? "rflags" => uint,                     ; u64; bit 1 MUST be 1; default 0x2
  ; GPRs below: all u64
  ? "rax" => uint, ? "rbx" => uint, ? "rcx" => uint, ? "rdx" => uint,
  ? "rsi" => uint, ? "rdi" => uint, ? "rbp" => uint,
  ? "r8"  => uint, ? "r9"  => uint, ? "r10" => uint, ? "r11" => uint,
  ? "r12" => uint, ? "r13" => uint, ? "r14" => uint, ? "r15" => uint,
  ; control registers and EFER: all u64
  ? "cr0"  => uint, ? "cr3" => uint, ? "cr4" => uint, ? "efer" => uint,
  ? "cs"   => seg-reg,
  ? "ds"   => seg-reg, ? "es" => seg-reg, ? "fs" => seg-reg,
  ? "gs"   => seg-reg, ? "ss" => seg-reg,
  ? "gdtr" => dtr,
  ? "idtr" => dtr,
}

seg-reg = {
  ? "selector"   => uint,                 ; u16
  ? "attributes" => uint,                 ; u16; encoding below
  ? "limit"      => uint,                 ; u32
  ? "base"       => uint,                 ; u64
}

dtr = {
  ? "limit" => uint,                      ; u16
  ? "base"  => uint,                      ; u64
}
```

`rflags` defaults to `0x2`. If specified, bit 1 MUST be 1.

#### Segment-register attributes encoding

| Bits    | Meaning                                                 |
| ------- | ------------------------------------------------------- |
| `0–3`   | Type (Intel SDM Vol. 3 §3.4.5.1 / AMD APM Vol. 2 §4.7). |
| `4`     | S — 0 = system, 1 = code/data.                          |
| `5–6`   | DPL — 0–3.                                              |
| `7`     | P.                                                      |
| `8`     | AVL.                                                    |
| `9`     | L — 64-bit code segment (CS only; ignored elsewhere).   |
| `10`    | D/B — 0 = 16/64-bit, 1 = 32-bit.                        |
| `11`    | G — 0 = byte, 1 = 4 KiB.                                |
| `12–15` | Reserved. MUST be zero.                                 |

### `vcpu-aarch64`

```cddl
vcpu-aarch64 = {
  ; x0..x30: all u64
  ? "x0"  => uint, ? "x1"  => uint, ? "x2"  => uint, ? "x3"  => uint,
  ? "x4"  => uint, ? "x5"  => uint, ? "x6"  => uint, ? "x7"  => uint,
  ? "x8"  => uint, ? "x9"  => uint, ? "x10" => uint, ? "x11" => uint,
  ? "x12" => uint, ? "x13" => uint, ? "x14" => uint, ? "x15" => uint,
  ? "x16" => uint, ? "x17" => uint, ? "x18" => uint, ? "x19" => uint,
  ? "x20" => uint, ? "x21" => uint, ? "x22" => uint, ? "x23" => uint,
  ? "x24" => uint, ? "x25" => uint, ? "x26" => uint, ? "x27" => uint,
  ? "x28" => uint, ? "x29" => uint, ? "x30" => uint,
  ? "sp_el1" => uint,                     ; u64
  ? "pc"     => uint,                     ; u64
  ? "pstate" => uint,                     ; u64; SPSR encoding below
  ; system registers below: all u64
  ? "sctlr_el1" => uint, ? "tcr_el1"   => uint,
  ? "ttbr0_el1" => uint, ? "ttbr1_el1" => uint,
  ? "mair_el1"  => uint, ? "vbar_el1"  => uint,
  ? "cpacr_el1" => uint,
}
```

System-register keys (`sctlr_el1` through `cpacr_el1`) follow the encodings in
the Arm Architecture Reference Manual for ARMv8-A and later.

#### pstate

| Bits    | Meaning                                                                 |
| ------- | ----------------------------------------------------------------------- |
| `0–3`   | M[3:0] — target exception mode. MUST select EL1 (e.g., `0x5` for EL1h). |
| `4`     | M[4] — execution state. MUST be 0 (AArch64).                            |
| `5`     | Reserved. MUST be zero.                                                 |
| `6`     | F — FIQ mask.                                                           |
| `7`     | I — IRQ mask.                                                           |
| `8`     | A — SError mask.                                                        |
| `9`     | D — debug mask.                                                         |
| `10–27` | Reserved or architecture-defined. See Arm ARM.                          |
| `28–31` | NZCV.                                                                   |
| `32–63` | Reserved or architecture-defined. See Arm ARM.                          |

The VMM MUST reject a `vm:vcpu` whose `pstate` selects an EL other than EL1.
