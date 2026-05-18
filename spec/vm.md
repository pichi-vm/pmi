# `vm` Target

The `vm` target is the non-CC virtual machine launch path. It defines the
**base launch model** for PMI; confidential targets ([`sev`](sev.md),
[`tdx`](tdx.md), [`cca`](cca.md)) inherit this model and layer their
cryptographic steps on top, describing only the deltas.

## PE section

A VMM targeting `vm` reads the `.pmi.vm` PE section. The section MUST be
non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). If the section is absent, the image
does not support `vm` and the VMM MUST refuse to launch.

## Schema

```cddl
vm = {
  "version"  => uint,                  ; schema version, currently 1
  "dtb"      => tstr,                  ; PE section name; see dtb.md
  "vcpu"     => vcpu-x64 / vcpu-aarch64, ; arch selected by PE.FileHeader.Machine; see vcpu below
  "actions"  => [+ vm-action],         ; ordered launch recipe (step 4)
}

vm-action = load / dtbo
```

VMMs MUST reject sections with an unrecognized `version`, an unknown
top-level key, or an unknown action `type` value.

## Launch model

A VMM executes the launch in six ordered steps:

1. **Select target.** Read the `.pmi.<target>` PE section. Refuse to
   launch if it is absent.
2. **Inspect DTB.** Parse the FDT named by the spec's [`dtb`](dtb.md)
   field and validate that the host can satisfy every hardware capability
   it declares. Fail the launch if any declaration cannot be satisfied.
3. **Target initialize.** No-op for `vm`. CC targets use this step to
   establish a cryptographic launch context; see each target binding.
4. **Process actions.** Process each entry in the `actions` array in
   order. Each action's `type` field selects how the VMM consumes it:
   - [`load`](load.md) — load the named PE section's bytes into guest
     memory at the section's `VirtualAddress`.
   - [`dtbo`](dtbo.md) — fill the named zero PE section with the runtime
     devicetree overlay (see [Runtime overlay](#runtime-overlay) below).
5. **Target finalize.** Apply the spec's [`vcpu`](#vcpu) register map to
   the boot vCPU. CC targets additionally use this step to seal the
   launch measurement; see each target binding.
6. **Start the guest.**

On CC targets, the launch-measurement API is fed in step-4 order, so
reordering actions produces a different digest.

## Runtime overlay

The runtime overlay described under [platform-definition
inversion](overview.md#solving-the-platform-definition-inversion) reaches
the guest through a [`dtbo`](dtbo.md) action processed at step 4. A
`dtbo` action names a zero PE section — a section that reserves a GPA
range but carries no on-disk data. The VMM generates the overlay for
this launch (the `/cpus`, `/memory@*`, and `/distance-map` subtrees,
plus any `numa-node-id` annotations on image-declared nodes) and writes
it into the reserved range. The overlay is generated fresh per launch
and is not measured; the guest is responsible for merging it onto the
base DTB before booting.

## `vcpu`

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
