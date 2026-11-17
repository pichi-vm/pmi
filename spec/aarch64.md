# aarch64

This document defines the parts of PMI specific to the aarch64 architecture. An
image is aarch64 when its `PE.FileHeader.Machine` is `0xAA64`.

## `cpu:profile`

The aarch64 values of the
[`cpu:profile`](cpu.md#1-new-target-attribute-cpuprofile) target attribute are
the A-profile architecture revisions of the [Arm Architecture Reference
Manual][arm-arm]:

- `armv8.0-a`
- `armv8.1-a`
- `armv8.2-a`
- `armv8.3-a`
- `armv8.4-a`
- `armv8.5-a`
- `armv8.6-a`
- `armv8.7-a`
- `armv8.8-a`
- `armv8.9-a`
- `armv9.0-a`
- `armv9.1-a`
- `armv9.2-a`
- `armv9.3-a`
- `armv9.4-a`
- `armv9.5-a`
- `armv9.6-a`
- `armv9.7-a`
- `armv9.8-a`

These profiles select the architectural minimum defined by the Arm ARM for
the listed revision, subject to the [shared profile
requirements](cpu.md#1-new-target-attribute-cpuprofile).

For example, [Arm 109697_2025_12_en, §9.11, page 119][arm-features] distinguishes
SVE2's optional architectural status from its provision in systems with rich
application environments. Selecting `armv9.0-a` alone does not require SVE2.

Armv9.x-A is not a strict superset of Armv8.(x+n)-A for n>0: each Armv9.x-A
revision is built on a fixed Armv8-A baseline (Armv9.0-A on Armv8.5-A,
Armv9.1-A on Armv8.6-A, Armv9.2-A on Armv8.7-A, and so on). A host satisfying
`armv9.0-a` therefore does not necessarily satisfy `armv8.6-a` or higher, and
vice versa. Image authors choosing between an `armv8.x-a` and `armv9.y-a`
profile should consult the Arm ARM for the relevant baseline.

## Boot vCPU

The aarch64 boot vCPU state is EL1 with the MMU and caches disabled, at address
`0x0`.

| Register                                      | Value                                                          |
| --------------------------------------------- | -------------------------------------------------------------- |
| `PC`                                          | `0x0`                                                          |
| `PSTATE`                                      | `0x3C5` in the SPSR encoding (EL1h; `D`, `A`, `I`, `F` masked) |
| `SCTLR_EL1`                                   | `M`, `C`, `I`, and `EE` clear; other bits unspecified          |
| general-purpose registers, `SP_EL0`, `SP_EL1` | `0`                                                            |

An image entered in this state MUST NOT rely on a field or bit this section
leaves unspecified, or on any register the table does not list.

## Guest physical address width

A guest learns its guest physical address (IPA) width from
`ID_AA64MMFR0_EL1.PARange`.

[arm-arm]: https://developer.arm.com/documentation/ddi0487/latest
[arm-features]: https://documentation-service.arm.com/static/69402e206efc1635355c3bb2
