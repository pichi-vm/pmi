# PMI Core Specification

Operating systems today can already boot on bare metal via UEFI by using the PE
format. An example of this is a Linux UKI. This does not require PMI.

PMI adds the ability to boot different kinds of VMs from the same binary. A bare
metal (UEFI) system will ignore all PMI-related data and sections. A PMI-aware
VM, however, will be able to locate and execute the PMI-defined actions to
successfully compose a virtual machine.

This page covers the core specification. However, most actual functionality is
defined as [extensions](extensions.md).

## Targets

A PMI **target** is a launch recipe - a CBOR-encoded specification, carried in a
`.pmi.<target>` PE section, that tells a VMM how to assemble and start a guest
VM. Different targets express different launch paths:

1. a traditional virtual machine
2. a confidential virtual machine on AMD SEV, Arm CCA or Intel TDX

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
   alignment — see [page granularity](constraints.md#page-granularity).

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
  "kind"    => tstr,                ; REQUIRED; no default
}
```

The PE `section` MUST be a Zero section (`SizeOfRawData == 0`,
`VirtualSize > 0`).

The `fill` action MUST include a `kind` value.

#### Procedure

1. The VMM locates the PE section with the same name as `section`.

2. The VMM allocates `VirtualSize` bytes of memory.

3. The VMM fills in the memory with content as defined by the `kind` value.

4. The VMM maps or copies the memory into the guest as the `VirtualAddress`
   (understood as GPA) location as defined by the `kind` value.

The VMM MAY break the range into a series of page-sized operations. When it does,
it MUST process them from the lowest GPA to the highest, so that any
order-sensitive target measurement is reproducible from the image bytes.

#### `kind`

The `kind` value determines the behavior of the `fill` action. However, the core
specification does not define any `kind` values.

The `kind` value is [extensible](extensions.md). Extensions MAY define
additional `kind` values. Extension-defined `kind` values MUST follow all
namespacing rules. This implies that, unless an extension defines a `fill`
value, the `fill` action cannot be used.
