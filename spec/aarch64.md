# aarch64

This document defines the parts of PMI specific to the aarch64 architecture. An
image is aarch64 when its `PE.FileHeader.Machine` is `0xAA64`.

## `cpu:profile`

The aarch64 values of the
[`cpu:profile`](cpu.md#1-new-target-attribute-cpuprofile) target attribute:

```cddl
profile-aarch64 = tstr                      ; Armv8-A / Armv9-A revision
```

A `profile-aarch64` value is an Armv8-A or Armv9-A architecture revision name
of the form `armvN.M-a` (e.g., `armv8.2-a`, `armv9.0-a`), as defined by the
[Arm Architecture Reference Manual for A-profile architecture][arm-arm]. The
set of mandatory features at each revision is fixed by the Arm ARM. Revisions
added by future editions of the Arm ARM are valid `cpu:profile` values without
updates to this specification.

Armv9.x-A is not a strict superset of Armv8.(x+n)-A for n>0: each Armv9.x-A
revision is built on a fixed Armv8-A baseline (Armv9.0-A on Armv8.5-A,
Armv9.1-A on Armv8.6-A, Armv9.2-A on Armv8.7-A, and so on). A host satisfying
`armv9.0-a` therefore does not necessarily satisfy `armv8.6-a` or higher, and
vice versa. Image authors choosing between an `armv8.x-a` and `armv9.y-a`
profile should consult the Arm ARM for the relevant baseline.

[arm-arm]: https://developer.arm.com/documentation/ddi0487/latest
