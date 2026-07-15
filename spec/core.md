# PMI Core Specification

PMI builds on the Portable Executable (PE) format. PE is already bootable on
bare metal under UEFI (a Linux UKI is one example), but PMI neither defines nor
depends on that path. PMI is a separate, additive layer. It adds non-loaded PE
sections that a PMI-aware VMM reads to compose a virtual machine, and that
non-PMI loaders ignore.

The two are independent. A PMI image can also be a UKI, but need not be; a UKI
can carry PMI, but need not. They are parallel, compatible extensions to the
same PE container.

This document defines the PMI core: the [target](#targets) shape, the
[launch model](#launch-model), the [validation rules](#validation), and the
[`load`](#load) and [`fill`](#fill) actions. Everything else, every launch
target and platform mechanism, is an [extension](extensions.md).

## Targets

A PMI **target** is a launch recipe: a CBOR-encoded specification, carried in a
`.pmi.<target>` PE section that MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`),
that tells a VMM how to assemble and start a guest VM. Different targets
express different launch paths:

1. a traditional virtual machine
2. a confidential virtual machine on AMD SEV, Arm CCA or Intel TDX

A single PMI image MAY support multiple targets, one `.pmi.<target>` section per
target; a VMM reads only the section for the target it launches. Distinct
targets MAY reference the same underlying PE sections, so the data a target
loads (a kernel, firmware, et cetera) can be shared across the targets an image
supports rather than duplicated per target.

### Shape

Every PMI **target** is a CBOR map that follows this shape:

```cddl
target = {
  "version" => uint,                       ; schema version
  "actions" => [+ action],                 ; ordered launch recipe
  ; per-target firmware-bound fields and extension attributes
}

action = {
  "type" => tstr,                          ; selects action type
  ; per-type fields
}
```

`type` is the only universal action field. Everything else is defined per action
type.

### Launch model

A VMM launches a target by executing this ordered sequence:

1. **Read `.pmi.<target>`.** Locate and decode the target's PE section. Refuse
   to launch if absent.
2. **Initialize.** Perform target-specific setup before processing actions
   (e.g., on confidential targets, call the CC firmware's launch-start API).
3. **Process actions.** Execute each entry in the `actions` array in array
   order. Each action's `type` selects the operation; the per-type fields
   parameterize it.
4. **Finalize.** Apply post-action state (e.g., write boot-vCPU registers,
   finalize the CC measurement).
5. **Start the guest.**

## Validation

A VMM MUST refuse to launch on any of:

- unrecognized `version`;
- unknown key in any CBOR map in the spec;
- unknown action `type`;
- any action's `section` does not name a PE section present in the image;
- two actions place overlapping guest-memory ranges, i.e. their
  `[gpa, gpa + VirtualSize)` ranges intersect.

Per-target specs MAY add further validation rules.

The overlap rule is scoped to the active target: actions in disjoint targets MAY
place sections at the same `gpa`. Only one target's spec is active per launch,
and the VMM reads only the `.pmi.<target>` section for its target, so a `gpa`
shared between actions in, say, `.pmi.sev` and `.pmi.tdx` can never collide in
guest memory.

## Actions

### Placement and `VirtualAddress`

Both [`load`](#load) and [`fill`](#fill) place bytes at an explicit, absolute
guest-physical address given by the action's `gpa`. PMI does not consult the PE
`VirtualAddress` of a referenced section, and applies no relocation: `gpa` is the
GPA verbatim. (`VirtualAddress` is a PE _relative_ virtual address, relative to
`ImageBase`, meaningful only to non-PMI loaders such as UEFI.)

This decoupling lets one PE serve two interpretations at once. A PMI image that
is _also_ a bootable UKI keeps a compact, relocatable PE layout for the UEFI path,
with its sections at modest `VirtualAddress`es, ASLR-relocated at load. PMI in
turn places those same sections at whatever GPAs the guest requires (for example
guest firmware near the 4 GiB reset vector) via `gpa`, with no effect on the PE
layout or `SizeOfImage`. Dual-use images SHOULD set `ImageBase` to 0; PMI ignores
`ImageBase` regardless.

The granularity rules ([page granularity](granularity.md)) and the overlap check
([Validation](#validation)) operate on `gpa`, not `VirtualAddress`.

### Measurement determinism

On targets that produce a launch measurement, the measurement MUST be a
deterministic function of the image bytes, namely each measured unit's content,
GPA, and page type, taken in a fixed total order, and MUST NOT depend on the
page size a VMM chooses to load or map guest memory with.

The total order is: actions in `actions` array order; within an action,
ascending GPA. Where a target's per-page submission involves more than one
measured sub-operation (e.g. TDX's page-add followed by content-extend), the
target's spec MUST additionally define the order of those sub-operations. Each
target's spec states its fixed measurement granularity and any such
sub-operation order.

### `load`

The `load` action loads a PE section's on-disk bytes into guest memory.

#### Schema

```cddl
load = {
  "type"    => "load",
  "gpa"     => uint,                ; absolute guest-physical address
  "section" => tstr,                ; PE section supplying the bytes
  ? "kind"  => tstr,                ; default: "default"
}
```

The `load` action MAY include a `kind` value. The `gpa` field gives the absolute
guest-physical address at which the bytes are placed; it is required. PMI does
not use the section's PE `VirtualAddress` for placement (see
[Placement and `VirtualAddress`](#placement-and-virtualaddress)).

#### Procedure

1. The VMM locates the PE section with the same name as `section`.

2. The VMM maps or copies the bytes from the PE section into guest memory at the
   action's `gpa`. The number of bytes placed is the section's `VirtualSize`. The
   section's `VirtualAddress` is not consulted (see
   [Placement and `VirtualAddress`](#placement-and-virtualaddress)). Note that
   the specific behavior of this operation is dictated by the `kind` value.

   The VMM reads the section's bytes from the file at `PointerToRawData`, so a
   referenced section need not be loaded by a non-PMI loader and MAY be marked
   `IMAGE_SCN_MEM_DISCARDABLE`.

   The VMM MAY break the section into page-sized operations and MAY choose any
   loading page size; that choice is an implementation detail and MUST NOT affect
   any launch measurement (see [Measurement determinism](#measurement-determinism)).

#### Section Shapes

There are three PE-section shapes:

1. **Data** (`SizeOfRawData > 0`, `VirtualSize == SizeOfRawData`). Load the
   on-disk data at `gpa`. The VMM chooses page granularity based on
   alignment (see [page granularity](granularity.md)).

2. **Padded** (`SizeOfRawData > 0`, `VirtualSize > SizeOfRawData`). Load the
   on-disk data at `gpa` as in the Data shape above. Then zero-fill
   from `gpa + SizeOfRawData` to `gpa + VirtualSize`. This
   mirrors standard PE `.bss`-tail behavior.

3. **Zero** (`SizeOfRawData == 0`, `VirtualSize > 0`). The entire region is
   zero-filled. No disk data is loaded. This is how reserved memory regions are
   expressed.

#### `kind`

The `kind` value determines the behavior of the `load` action. If `kind` is
omitted, `default` is assumed. However, the core specification does not define
any behavior for `kind = "default"`.

The `kind` value is [extensible](extensions.md). Extension-defined targets MUST
define the behavior of the `load` action when `kind = "default"`. Extensions MAY
define additional `kind` values. Extension-defined `kind` values MUST follow all
namespacing rules. A VMM MUST refuse to launch on a `load` whose `kind` it does
not recognize.

### `fill`

The `fill` action populates a reserved GPA range at launch with kind-specific
content.

#### Schema

```cddl
fill = {
  "type"    => "fill",
  "gpa"     => uint,                ; absolute guest-physical address
  "section" => tstr,                ; zero PE section to populate
  "kind"    => tstr,                ; selects fill kind
}
```

The `fill` action MUST include a `kind` value, and a required `gpa` giving the
absolute guest-physical address of the filled range. As with `load`, PMI does
not use the section's `VirtualAddress` for placement (see
[Placement and `VirtualAddress`](#placement-and-virtualaddress)).

#### Procedure

1. The VMM locates the PE section with the same name as `section`.

2. The VMM allocates `VirtualSize` bytes of memory and fills it with content as
   defined by the `kind` value, then maps or copies it into the guest at the
   action's `gpa`. As with `load`, the section's `VirtualAddress` is not used by
   PMI.

   The VMM MAY break the range into page-sized operations and MAY choose any
   loading page size; that choice is an implementation detail and MUST NOT affect
   any launch measurement (see [Measurement determinism](#measurement-determinism)).

#### Section Shape

The referenced PE section MUST be a Zero section (`SizeOfRawData == 0`,
`VirtualSize > 0`); the fill content comes from the `kind`, not from disk.

#### `kind`

The `kind` value determines the behavior of the `fill` action. It has no
default; every `fill` action MUST carry a `kind`. The `kind` also determines the
memory class of the placement, private (encrypted/integrity-protected) or
shared guest memory, which matters on confidential targets (e.g. `dt:dtbo`
is unmeasured-private where supported, otherwise shared; see [`dt`](dt.md)).

The `kind` value is [extensible](extensions.md). Extensions MAY define `kind`
values. Extension-defined `kind` values MUST follow all namespacing rules. A VMM
MUST refuse to launch on a `fill` whose `kind` it does not recognize.

## Measured vs. host-controlled inputs

A PMI requirement on the VMM falls into one of two classes, and guests MUST treat
them differently.

**Measured inputs** fold into the launch measurement (the bytes placed by
`load`, and each target's measured launch parameters). A deviation changes the
measurement and is caught at attestation, so a guest backed by a remote
verifier checking the measurement may rely on them. A measured input is
usually fixed by the image, but need not be: a host MAY supply its content, for
example a substituted [`dt:dtb`](dt.md) base, and it remains measured and
attested. Reliance then rests on the verifier appraising the measurement against
an expected value, which is predictable only when that value is authored by a
trusted party (see [`dt` authorship](dt.md#authorship-and-attestation-predictability)).

**Host-controlled, unmeasured inputs** are those a VMM supplies that do not enter
the launch measurement: resource allocation (the [`dt`](dt.md) overlay),
and per-target launch configuration such as TDX `TD_PARAMS` (including `XFAM` and
`CPUID_VALUES`), SEV's launch policy, `host_data`, and CPUID-page contents, the
unmeasured `RmiRealmParams` subset on CCA, and the initial register state where
the platform fixes it. For these, a "the VMM MUST …" requirement in this
specification describes a _conformant_ host; the launch measurement does not
enforce it, and a non-conformant or malicious host can violate it undetected by
the measurement.

An input is permitted to be host-controlled and unmeasured only when a host
deviation can cause at most denial of service, which a host can always
inflict regardless. Anything a host could exploit _beyond_ denial of service MUST
be either measured, attested in a report field a remote verifier checks, or
validated by the guest. Accordingly:

- **Denial-of-service-only** deviations need no guest defense: failing is itself
  the denial of service. A guest MAY verify such properties for clearer
  diagnostics, but is not required to.
- **Guest-validated** inputs (the overlay) MUST be validated and the launch
  rejected on violation (see [`dt`](dt.md)).
- **Verifier-checked** inputs (e.g. SEV launch policy, TDX `tdx_xfam` /
  `tdx_td_attributes`) MUST be checked by the remote verifier in the attestation
  report; the launch measurement alone does not establish them.

A guest MUST NOT assume an unmeasured "MUST" was honored. Where it depends on an
unmeasured property beyond denial of service, it MUST verify it and fail safe,
drawing on an authoritative architectural source: `CPUID` and the ID registers
(`MIDR_EL1`, `ID_AA64*`), `TDCALL[TDG.VP.INFO]`, `RSI_REALM_CONFIG`, or the
validated overlay.
