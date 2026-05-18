# `vm` Target

The `vm` target is the non-CC virtual machine launch path. The VMM reads the
image's base [DTB](dtb.md), processes the actions list to load guest memory
and apply any host overlay, sets boot-vCPU register state, and starts the
guest.

## PE section

A VMM targeting `vm` reads the `.pmi.vm` PE section. The section MUST be
non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is absent, the image
does not support `vm` and the VMM MUST refuse to launch.

## Schema

```cddl
vm = {
  "version"  => uint,                  ; schema version, currently 1
  "dtb"      => tstr,                  ; PE section name; see dtb.md
  "actions"  => [+ vm-action],         ; ordered launch recipe
  * tstr => any,                       ; unknown keys ignored
}

vm-action = load / dtbo / vcpu
```

VMMs MUST reject sections with an unrecognized `version`.

The `actions` array is processed in order. Each action's `type` selects its
schema:

- [`load`](load.md) — load a PE section's bytes into guest memory
- [`dtbo`](dtbo.md) — write the host-decided devicetree overlay
- [`vcpu`](#vcpu-action) — set boot-vCPU register state (defined below)

The `vm` `actions` array MUST contain at least one `vcpu` action. If multiple
are present, the VMM MUST use the last one in array order; earlier `vcpu`
actions are ignored.

Consumers MUST ignore unknown keys but MUST reject unknown action `type`
values.

## `vcpu` action

The `vcpu` action carries a CBOR-encoded map of register values for the boot
vCPU. The VMM decodes the referenced PE section's on-disk bytes as CBOR, looks
up each key in the architecture-specific schema selected by the PE header's
`FileHeader.Machine` field, and applies the corresponding values to the boot
vCPU before starting the guest. Other vCPUs start in their architecture-defined
reset state; the boot vCPU is responsible for bringing them up.

### Schema

```cddl
vcpu = {
  "type"    => "vcpu",
  "section" => tstr,                ; PE section containing the CBOR register map
}
```

The referenced PE section's contents are consumed only by the VMM; the bytes
MUST NOT be written to guest memory. The PE section MUST be non-loaded
(`IMAGE_SCN_MEM_DISCARDABLE`) so that UEFI loaders also skip it. The PE
section's `VirtualAddress` field has no semantic meaning for `vcpu`; its
content is the CBOR blob occupying `SizeOfRawData` bytes at `PointerToRawData`.

Missing keys in the register map default to zero (with the per-architecture
exceptions noted below). Unknown keys MUST be ignored.

The VMM MUST reject a `vcpu` register map where any value exceeds the field
width defined by the architecture schema (e.g., a `selector` value greater
than `0xFFFF`).

### x86-64 (`PE.FileHeader.Machine == 0x8664`)

```cddl
vcpu-x64 = {
  ? "rip"    => uint,                     ; u64
  ? "rsp"    => uint,                     ; u64
  ? "rflags" => uint,                     ; u64; bit 1 MUST be 1; default 0x2
  ? "rax" => uint, ? "rbx" => uint, ? "rcx" => uint, ? "rdx" => uint,
  ? "rsi" => uint, ? "rdi" => uint, ? "rbp" => uint,
  ? "r8"  => uint, ? "r9"  => uint, ? "r10" => uint, ? "r11" => uint,
  ? "r12" => uint, ? "r13" => uint, ? "r14" => uint, ? "r15" => uint,
  ? "cr0"  => uint, ? "cr3" => uint, ? "cr4" => uint, ? "efer" => uint,
  ? "cs"   => seg-reg,
  ? "ds"   => seg-reg, ? "es" => seg-reg, ? "fs" => seg-reg,
  ? "gs"   => seg-reg, ? "ss" => seg-reg,
  ? "gdtr" => dtr,
  ? "idtr" => dtr,
  * tstr => any,
}

seg-reg = {
  ? "selector"   => uint,                 ; u16
  ? "attributes" => uint,                 ; u16; encoding below
  ? "limit"      => uint,                 ; u32
  ? "base"       => uint,                 ; u64
  * tstr => any,
}

dtr = {
  ? "limit" => uint,                      ; u16
  ? "base"  => uint,                      ; u64
  * tstr => any,
}
```

GPR, control-register, RIP/RSP/RFLAGS, segment-register, and
descriptor-table-register keys correspond to the architecture-named registers.
CR2, TR, LDTR, debug registers, floating-point state, and MSRs other than EFER
are not specified by `vcpu` and start in their architecture-defined reset
state. The guest is responsible for initializing them as needed.

`rflags` defaults to `0x2` if omitted (bit 1 set, all other bits clear). If
specified, bit 1 MUST be 1.

#### Segment-register attributes encoding

| Bits    | Meaning                                                     |
| ------- | ----------------------------------------------------------- |
| `0–3`   | Type (see Intel SDM Vol. 3 §3.4.5.1 / AMD APM Vol. 2 §4.7). |
| `4`     | S — descriptor class: 0 = system, 1 = code/data.            |
| `5–6`   | DPL — descriptor privilege level (0–3).                     |
| `7`     | P — present.                                                |
| `8`     | AVL — available for software use.                           |
| `9`     | L — 64-bit code segment (CS only; ignored elsewhere).       |
| `10`    | D/B — default operation size (0 = 16/64-bit, 1 = 32-bit).   |
| `11`    | G — granularity: 0 = byte, 1 = 4 KiB.                       |
| `12–15` | Reserved. MUST be zero.                                     |

A typical 64-bit code segment has `attributes = 0x209B`: type = `0xB` (code,
readable, accessed), S = 1, DPL = 0, P = 1, L = 1. A typical 64-bit data
segment has `attributes = 0x0093`: type = `0x3` (data, writable, accessed),
S = 1, DPL = 0, P = 1.

### aarch64 (`PE.FileHeader.Machine == 0xAA64`)

```cddl
vcpu-aarch64 = {
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
  ? "pstate" => uint,                     ; u64; SPSR encoding (see below)
  ? "sctlr_el1" => uint, ? "tcr_el1"   => uint,
  ? "ttbr0_el1" => uint, ? "ttbr1_el1" => uint,
  ? "mair_el1"  => uint, ? "vbar_el1"  => uint,
  ? "cpacr_el1" => uint,
  * tstr => any,
}
```

GPR, PC, and SP_EL1 keys correspond to the architecture-named registers. The
system-register keys (`sctlr_el1` through `cpacr_el1`) follow the encodings
defined in the Arm Architecture Reference Manual for ARMv8-A and later. The
image MAY omit them, in which case the guest enters with MMU disabled and the
kernel configures them — this matches the Linux aarch64 boot protocol.

Debug registers, FPU/SIMD state, system registers other than those listed
above (including `spsr_el1`, `elr_el1`, `tpidr_el*`, `cntv_*`,
pointer-authentication keys, and read-only ID registers) are not specified by
`vcpu` and start in their architecture-defined reset state. The guest is
responsible for initializing them as needed.

#### pstate

`pstate` uses the standard AArch64 SPSR encoding:

| Bits    | Meaning                                                                 |
| ------- | ----------------------------------------------------------------------- |
| `0–3`   | M[3:0] — target exception mode. MUST select EL1 (e.g., `0x5` for EL1h). |
| `4`     | M[4] — execution state. MUST be 0 (AArch64).                            |
| `6`     | F — FIQ mask.                                                           |
| `7`     | I — IRQ mask.                                                           |
| `8`     | A — SError mask.                                                        |
| `9`     | D — debug mask.                                                         |
| `28–31` | NZCV condition flags.                                                   |

Other PSTATE bits follow the Arm ARM. A typical kernel-entry value is `0x3C5`
(EL1h, all DAIF masked, condition flags clear).

The VMM MUST reject a `vcpu` whose `pstate` selects an EL other than EL1.
EL2 entry is not supported by `vcpu` v1; HVF on Apple Silicon does not
expose EL2 to guests, and EL1 entry works on KVM as well.
