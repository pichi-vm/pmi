# `cpu` Extension

**Prefix:** `cpu`.

The `cpu` extension defines a portable declaration of the vCPU ISA baseline the
guest requires. It defines one extension point:

1. The new target attribute [`cpu:profile`](#1-new-target-attribute-cpuprofile).

Targets opt in by listing `cpu:profile` as a required key in their target spec.

## 1. New target attribute: `cpu:profile`

`cpu:profile` names the ISA baseline the guest is built against. Each
[architecture's document](core.md#architectures) lists the values valid for that
architecture.

```cddl
cpu-profile = tstr
```

A VMM MUST refuse to launch on any of:

- the value is not listed by the document for the image's
  `PE.FileHeader.Machine`;
- the host cannot deliver every feature mandated by the requested profile.

The profile's defining specification determines its mandatory features,
including conditional requirements whose conditions hold for the guest-visible
CPU. The VMM MUST check each mandatory feature individually. A host's
self-claimed architecture revision is not authoritative evidence that those
features are available to the guest.

The profile requires the minimum defined by that specification. It does not
implicitly add requirements from a system architecture, operating-system
environment, or other software baseline beyond its definition. Features that
are optional under that definition remain optional unless an applicable
condition requires them.

### Floor and measured ceiling

The profile is a **floor** the VMM MUST always honor: every feature mandated by
the requested profile MUST be exposed to the guest.

The floor is unmeasured on targets where the profile-derived configuration does
not enter the launch measurement (e.g. TDX `XFAM`/`CPUID_VALUES`, the SEV CPUID
page contents). There a non-conformant host can expose fewer features than the
profile, but the only consequence is the guest's own boot failure, a denial of
service, which is why the configuration is permitted to be unmeasured (see
[Measured vs. host-controlled
inputs](core.md#measured-vs-host-controlled-inputs)). A guest therefore does not
rely on the floor and need not verify it. A host cannot _over_-claim features it
lacks, and the security-relevant subset is captured by measurement or
attestation; each target's `cpu:profile` section gives the mechanism.

On targets where the profile-derived vCPU configuration enters a launch
measurement, the profile is also a **ceiling** on the measured fields: the VMM
MUST configure those fields as a deterministic function of the profile alone
(no fewer features, no more) so the measurement remains portable across
compliant VMMs, per the [core attestation
invariant](motivation.md#2-portable-safe-platform-definition-and-attestation).

On targets where the configuration does not enter a launch measurement, the VMM
MAY expose additional host-supported features beyond the profile.

A target MAY expose platform-forced features the VMM cannot mask; those are
exposed regardless of profile and remain visible to remote verifiers via the
target's separately-attested report fields.

Each target's spec defines how the VMM translates `cpu:profile` into the
target's firmware ABI inputs.
