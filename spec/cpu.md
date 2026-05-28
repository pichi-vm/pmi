# `cpu` Extension

**Prefix:** `cpu`.

The `cpu` extension defines a portable declaration of the vCPU ISA baseline the
guest requires. It defines one extension point:

1. The new target attribute [`cpu:profile`](#1-new-target-attribute-cpuprofile).

Targets opt in by listing `cpu:profile` as a required key in their target spec.

## 1. New target attribute: `cpu:profile`

`cpu:profile` names the ISA baseline the guest is built against, drawn from a
small per-architecture set. The schema is selected by `PE.FileHeader.Machine`:

- [`profile-x64`](#profile-x64) for `0x8664`,
- [`profile-aarch64`](#profile-aarch64) for `0xAA64`.

```cddl
cpu-profile = profile-x64 / profile-aarch64
```

A VMM MUST refuse to launch on any of:

- the `cpu:profile` variant does not match `PE.FileHeader.Machine`;
- the VMM does not recognize the requested profile;
- the host cannot deliver every feature mandated by the requested profile.

When determining whether the host satisfies the requested profile, the VMM MUST
check each mandatory feature individually. A host's self-claimed architecture
revision is not authoritative: some shipping cores (e.g., Apple M4, Qualcomm
Snapdragon 8 Gen 2/3) self-identify as Armv9-class while omitting mandatory
features such as SVE/SVE2.

### Floor and measured ceiling

The profile is a **floor** the VMM MUST always honor: every feature mandated by
the requested profile MUST be exposed to the guest.

On targets where the profile-derived vCPU configuration enters a launch
measurement, the profile is also a **ceiling** on the measured fields: the VMM
MUST configure those fields as a deterministic function of the profile alone —
no fewer features, no more — so the measurement remains portable across
compliant VMMs, per the [core attestation
invariant](motivation.md#2-portable-safe-platform-definition-and-attestation).

On targets where the configuration does not enter a launch measurement, the VMM
MAY expose additional host-supported features beyond the profile.

A target MAY expose platform-forced features the VMM cannot mask; those are
exposed regardless of profile and remain visible to remote verifiers via the
target's separately-attested report fields.

Each target's spec defines how the VMM translates `cpu:profile` into the
target's firmware ABI inputs.

### `profile-x64`

```cddl
profile-x64 = tstr                          ; x86-64 microarchitecture level
```

A `profile-x64` value is an x86-64 microarchitecture level name of the form
`x86-64-vN` (e.g., `x86-64-v3`), as defined by the [System V x86-64
psABI][sysv-abi]. The mandatory feature set at each level is fixed by the
psABI; each level is a strict superset of the one below. Levels added by future
revisions of the psABI are valid `cpu:profile` values without updates to this
specification.

### `profile-aarch64`

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

[sysv-abi]: https://gitlab.com/x86-psABIs/x86-64-ABI
[arm-arm]: https://developer.arm.com/documentation/ddi0487/latest
