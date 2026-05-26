# PMI Core Specification

PMI builds on the Portable Executable (PE) format. PE is already bootable on
bare metal under UEFI — a Linux UKI is one example — but PMI neither defines nor
depends on that path. PMI is a separate, additive layer: it adds non-loaded PE
sections that a PMI-aware VMM reads to compose a virtual machine, which non-PMI
loaders ignore.

The two are independent. A PMI image can also be a UKI, but need not be; a UKI
can carry PMI, but need not. They are parallel, compatible extensions to the
same PE container.

This document defines the PMI core: the [target](#targets) shape, the
[launch model](#launch-model), the [validation rules](#validation), and the
[`load`](#load) and [`fill`](#fill) actions. Everything else — every launch
target and platform mechanism — is an [extension](extensions.md).

## Targets

A PMI **target** is a launch recipe - a CBOR-encoded specification, carried in a
`.pmi.<target>` PE section, that tells a VMM how to assemble and start a guest
VM. Different targets express different launch paths:

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

1. **Read `.pmi.<target>`.** Locate and decode the target's PE section, which
   MUST be non-loaded (`IMAGE_SCN_MEM_DISCARDABLE`). Refuse to launch if absent.
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
- two action-referenced PE sections have overlapping
  `[VirtualAddress, VirtualAddress + VirtualSize)` ranges.

Per-target specs MAY add further validation rules.

## Actions

### `load`

The `load` action loads a PE section's on-disk bytes into guest memory.

#### Schema

```cddl
load = {
  "type"    => "load",
  "section" => tstr,                ; PE section name to load
  ? "kind"  => tstr,                ; default: "default"
}
```

The `load` action MAY include a `kind` value.

#### Procedure

1. The VMM locates the PE section with the same name as `section`.

2. The VMM maps or copies the bytes from the PE section into the guest memory.
   The section's `VirtualAddress` and `VirtualSize` should be understood as
   guest physical address (GPA) and size, respectively. Note that the specific
   behavior of this operation is dictated by the `kind` value.

   The VMM MAY break the section into a series of page-sized operations (for
   example, to load each page through a target API). When it does, it MUST
   process them from the lowest GPA to the highest, so that any order-sensitive
   target measurement is reproducible from the image bytes.

#### Section Shapes

There are three PE-section shapes:

1. **Data** (`SizeOfRawData > 0`, `VirtualSize == SizeOfRawData`). Load the
   on-disk data at `VirtualAddress`. The VMM chooses page granularity based on
   alignment — see [page granularity](granularity.md).

2. **Padded** (`SizeOfRawData > 0`, `VirtualSize > SizeOfRawData`). Load the
   on-disk data at `VirtualAddress` as in the Data shape above. Then zero-fill
   from `VirtualAddress + SizeOfRawData` to `VirtualAddress + VirtualSize`. This
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
namespacing rules.

### `fill`

The `fill` action populates a reserved GPA range at launch with kind-specific
content.

#### Schema

```cddl
fill = {
  "type"    => "fill",
  "section" => tstr,                ; zero PE section to populate
  "kind"    => tstr,                ; selects fill kind
}
```

The `fill` action MUST include a `kind` value.

#### Procedure

1. The VMM locates the PE section with the same name as `section`.

2. The VMM allocates `VirtualSize` bytes of memory and fills it with content as
   defined by the `kind` value, then maps or copies it into the guest at
   `VirtualAddress` (understood as GPA).

   The VMM MAY break the range into a series of page-sized operations. When it
   does, it MUST process them from the lowest GPA to the highest, so that any
   order-sensitive target measurement is reproducible from the image bytes.

#### Section Shape

The referenced PE section MUST be a Zero section (`SizeOfRawData == 0`,
`VirtualSize > 0`); the fill content comes from the `kind`, not from disk.

#### `kind`

The `kind` value determines the behavior of the `fill` action. The core
specification defines one `kind`, `dtb` (below); it has no default.

The `kind` value is [extensible](extensions.md). Extensions MAY define
additional `kind` values. Extension-defined `kind` values MUST follow all
namespacing rules.

The `dtb` kind delivers the guest's platform description (memory map, MMIO/IO
regions, CPU topology). The VMM fills the section with a host-supplied flattened
devicetree blob (DTB), in the format defined by the [Devicetree
Specification][devicetree] v0.4 or later. The host selects the DTB content via
VMM-defined input, out of scope for PMI. The DTB is **unmeasured** — it does not
contribute to the target's launch measurement — so the guest MUST validate it
before relying on it; the validation policy is the guest's and is out of scope
for this spec. For the rationale, see
[Motivation §2](motivation.md#2-portable-safe-platform-definition-and-attestation).

[devicetree]: https://www.devicetree.org/specifications/
