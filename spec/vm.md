# `vm` Extension

**Prefix:** `vm`.

The `vm` extension provides the essential functionality for launching a PMI as a
traditional virtual machine. It defines one extension point:

1. The new target [`.pmi.vm`](#1-new-target-pmivm).

## 1. New target: `.pmi.vm`

The `.pmi.vm` PE section carries the `vm` target spec, subject to the
[page granularity](granularity.md) rules.

### Launch model

The VMM executes the launch in five ordered steps:

1. Read the `.pmi.vm` PE section.
2. Initialize hypervisor state.
3. Process each entry in `actions` in array order.
4. Initialize the boot vCPU to its architecture's boot vCPU state (see
   [x86-64](x86-64.md#boot-vcpu) and [aarch64](aarch64.md#boot-vcpu)).
5. Start the guest.

### Keys

The `.pmi.vm` CBOR map follows the [core target shape](core.md#shape). Its
`version` MUST be `1`. It adds one required key:

- **`cpu:profile`** — vCPU ISA baseline (see [cpu.md](cpu.md)).

### Validation

The [core validation rules](core.md#validation) apply. In addition, the VMM MUST
refuse to launch if `PE.FileHeader.Machine` is neither `0x8664` nor `0xAA64`.

### `load`

On `vm`, the [`default`](core.md#kind) kind places the section's bytes in guest
memory per [section shape](core.md#section-shapes); no measurement is performed.
Implementations MAY copy or map the contents into guest memory.

### `fill`

Both [measured and unmeasured fills](core.md#measurement) place their supplied
bytes in ordinary guest RAM. Implementations MAY copy or map the contents.
The `vm` target produces no launch measurement.

### `cpu:profile`

The VMM configures the boot vCPU via the host hypervisor's facilities (e.g.,
KVM's `KVM_SET_CPUID2` on x86-64; `KVM_ARM_VCPU_INIT` feature bits and ID
register writes on aarch64) so the guest sees at least the profile. The `vm`
target has no launch measurement; the VMM MAY pass additional host-supported
features through to the guest. With no attestation, the measured/host-controlled
distinction does not apply to `vm`: the guest cannot, and need not, defend
against the host here.

## Example

An x86-64 `.pmi.vm` that loads boot code at the reset vector, a kernel, initrd,
and command line, and supplies a host devicetree:

```cbor-diag
{
  "version": 1,
  "cpu:profile": "x86-64-v3",
  "dt:dtb": ".dtb",
  "actions": [
    {"type": "load", "gpa": 0xFFFF0000, "section": ".boot"},
    {"type": "load", "gpa": 0x100000,   "section": ".linux"},
    {"type": "load", "gpa": 0x1000000,  "section": ".initrd"},
    {"type": "load", "gpa": 0x2000000,  "section": ".cmdline"},
    {"type": "load", "gpa": 0x2001000,  "section": ".dtb"},
    {"type": "fill", "gpa": 0x2011000,  "section": ".dtbo", "kind": "dt:dtbo"}
  ]
}
```

The VMM reads `.pmi.vm` and processes the actions in order. It loads `.boot`,
`.linux`, `.initrd`, `.cmdline`, and the base `.dtb` into guest memory, then
fills `.dtbo` with the host-supplied overlay the guest will merge onto the base.
It starts the boot vCPU in the [x86-64 boot vCPU state](x86-64.md#boot-vcpu),
where `.boot` enters 64-bit mode and boots the kernel. The same image might boot
on bare metal under UEFI as a UKI, ignoring `.pmi.vm` entirely.
