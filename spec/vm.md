# `vm` Target

The `vm` target is the non-CC virtual machine launch path. It defines
the **base launch model** that other PMI targets inherit.

## PE section

A VMM targeting `vm` reads the `.pmi.vm` PE section. The section MUST be
non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is absent, the image
does not support `vm` and the VMM MUST refuse to launch.

## Schema

```cddl
vm = {
  "version"  => uint,                  ; schema version (1)
  "vcpu"     => vcpu-x64 / vcpu-aarch64, ; selected by PE.FileHeader.Machine
  "actions"  => [+ vm-action],         ; ordered launch recipe (step 3)
}

vm-action = load / fill
```

VMMs MUST reject sections with an unrecognized `version`, an unknown
key in any defined CBOR map (top-level or nested), an unknown action
`type` value, or an unknown action `kind` value.

VMMs MUST additionally refuse to launch if:

- any action's `section` does not name a PE section present in the
  image;
- the same PE section name is referenced by more than one action; or
- two action-referenced PE sections have overlapping
  `[VirtualAddress, VirtualAddress + VirtualSize)` ranges.

## Launch model

A VMM executes the launch in five ordered steps:

1. **Select target.** Read the `.pmi.<target>` PE section. Refuse to
   launch if it is absent.
2. **Target initialize.** No-op.
3. **Process actions.** Process each entry in the `actions` array in
   order. Each action's `type` selects [`load`](#load-action) or
   [`fill`](#fill-action); the `kind` field selects the variant within
   that type.
4. **Target finalize.** Apply the spec's [`vcpu`](#vcpu-field) register
   map to the boot vCPU.
5. **Start the guest.**

Upper layers that need host-conformance checks, platform metadata
inspection, or other launch-time hooks beyond firmware-bound
operations carry their data and actions through the
[Extensions](overview.md#extensions) namespace; PMI does not
mandate those checks.

## `load` action

The `load` action loads a PE section's on-disk bytes into guest memory
at the section's `VirtualAddress`. The VMM reads `VirtualAddress`,
`SizeOfRawData`, `VirtualSize`, and `PointerToRawData` from the PE
section header.

### Schema

```cddl
load = {
  "type"    => "load",
  "section" => tstr,                ; PE section name to load
  ? "kind"  => "unmeasured",        ; vm defines one kind; default "unmeasured"
}
```

### Section shapes

There are three PE-section shapes:

1. **Data** (`SizeOfRawData > 0`, `VirtualSize == SizeOfRawData`). Load
   the on-disk data at `VirtualAddress`. The VMM chooses page granularity
   based on alignment — see [page granularity](pe.md#page-granularity).
2. **Padded** (`SizeOfRawData > 0`, `VirtualSize > SizeOfRawData`). Load
   the on-disk data at `VirtualAddress` as in the Data shape above. Then
   zero-fill from `VirtualAddress + SizeOfRawData` to
   `VirtualAddress + VirtualSize`. This is standard PE `.bss`-tail
   behavior — firmware or service modules that need reserved memory
   beyond their code use this to express it without file backing.
3. **Zero** (`SizeOfRawData == 0`, `VirtualSize > 0`). The entire region
   is zero-filled. No disk data is loaded. This is how reserved memory
   regions are expressed.

### kind `unmeasured`

The only load kind vm defines. The VMM places the bytes in guest memory
per the section shape; no measurement happens (vm is non-CC). This is
the default kind for vm's load and is omitted from the wire format.

Confidential targets that inherit vm's `load` action layer on
additional kinds with their own measurement and firmware-API semantics.
See those targets' bindings.

## `fill` action

The `fill` action populates a reserved GPA range at launch with
kind-specific content. The PE section MUST be a zero section
(`SizeOfRawData == 0`, `VirtualSize > 0`) — it reserves the address
range but carries no on-disk data.

### Schema

```cddl
fill = {
  "type"    => "fill",
  "section" => tstr,                ; zero PE section to populate
  "kind"    => tstr,                ; kind selector (per-target or namespaced)
}
```

The fill action MUST include a `kind` value; there is no default.
`vm` defines no fill kinds itself; CC targets layer firmware-bound
kinds on top (see [`sev`](sev.md#fill-action) for `secrets` and
`cpuid`), and upper layers register their own under the
[Extensions](overview.md#extensions) namespace.

## `vcpu` field

The `vcpu` field carries a CBOR-encoded map of register values for the
boot vCPU, inline in the target spec. The VMM looks up each key in the
architecture-specific schema selected by the PE header's
`FileHeader.Machine` field, and applies the corresponding values to the
boot vCPU at step 5 (finalize) before starting the guest. Other vCPUs
start in their architecture-defined reset state; the boot vCPU is
responsible for bringing them up.

Missing keys in the register map default to zero (with the
per-architecture exceptions noted below). The VMM MUST reject unknown
keys.

The VMM MUST reject a `vcpu` register map where any value exceeds the
field width defined by the architecture schema (e.g., a `selector` value
greater than `0xFFFF`).

### vcpu-x64

Used when `PE.FileHeader.Machine == 0x8664`.

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

### vcpu-aarch64

Used when `PE.FileHeader.Machine == 0xAA64`.

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
