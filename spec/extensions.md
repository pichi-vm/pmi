# Extensions

PMI is a substrate. The PE container, the per-target CBOR launch
recipes, and the action mechanism cover the firmware-bound mechanics
of every launch — and stop there. Upper layers (hypervisor specs
like dillo, image schemas, in-guest stubs) need to attach
layer-specific data to PMI images and have a VMM that understands
them do something with it. PMI exposes a single extensibility
contract — a namespacing convention — and three extension points
that bolt new behavior onto a target spec without changing PMI's
core shape.

## Common target shape

Every PMI target spec is a CBOR map with this skeleton:

```cddl
target = {
  "version" => uint,                       ; schema version
  "actions" => [+ action],                 ; ordered launch recipe
  ; per-target firmware-bound fields (e.g., sev's `id`, vm/cca's `vcpu`)
}

action = {
  "type"     => tstr,                      ; selects load / fill / ...
  "section"  => tstr,                      ; PE section the action applies to
  ; per-type fields (e.g., `kind` for load and fill)
}
```

Across `vm`, `sev`, `cca`, and `tdx` the shape never changes; only
the type-specific kinds, the firmware-bound side fields, and the
ABI the actions drive change. An upper layer extends this shape
without re-specifying it.

## Namespacing rule

PMI's only extensibility primitive: a namespacing convention for
names that appear in the wire format.

- **Unprefixed names** (e.g., `version`, `actions`, `secrets`,
  `cpuid`) are reserved for PMI.
- **Prefixed names** of the form `<layer>:<name>` (e.g.,
  `dillo:dtb`, `dillo:dtbo`, `dillo:configure`) belong to the named
  upper layer.
- The prefix names the **consumer** — typically a hypervisor or
  in-guest stub — not the producer. Multiple image tools may emit
  the same prefix for the same consumer.
- Loaders MUST reject any name they do not understand. A pure PMI
  loader presented with any `dillo:*` name refuses to launch; a
  `dillo`-aware loader handles it per dillo's spec.

The same strict-rejection rule that already governs unknown PMI
keys, types, and kinds handles unknown namespaced names with no
new mechanism.

## Three extension points

An upper layer attaches behavior at one of three places in a target
spec.

### 1. Target attributes (top-level keys)

A namespaced top-level CBOR key adds a piece of metadata the upper
layer needs to know about, independent of any action.

```cbor-diag
{
  "version": 1,
  "actions": [ ... ],
  "dillo:dtb": ".dillo.dtb",      ; extension: names a PE section
  "dillo:platform": ".dillo.cfg"  ; extension: more layer-specific metadata
}
```

The value can be any CBOR type the upper layer specifies. When it
points at a PE section (the common case for binary blobs), the
upper layer documents the section's expected contents.

PMI ignores everything under a prefix it doesn't own; a
dillo-aware loader reads `dillo:*` keys per dillo's spec.

### 2. New actions

A namespaced `type` value adds a new kind of operation the upper
layer wants performed at launch, alongside PMI's own `load` and
`fill` actions.

```cbor-diag
{
  "type": "dillo:configure",
  "section": ".dillo.cfg",
  ; per-action fields the upper layer defines
}
```

PMI executes actions in array order; an upper-layer action runs at
its array position relative to PMI's actions. The upper layer's
spec defines what the action does — what bytes (if any) the VMM
loads, what firmware or VMM calls happen, what measurement the
operation contributes to (if any).

The action's `section` follows the same rules as PMI actions: it
MUST name a PE section, MUST NOT duplicate another action's
section, and MUST NOT overlap another loaded section in guest
memory. The upper layer's spec defines any additional per-action
constraints.

### 3. Action customization (per-action kind)

For actions whose schema has a `kind` field — `load`, `fill`, and
future actions following the same pattern — a namespaced `kind`
value adds a new variant of that action without inventing a new
action type. This is the natural extension point for behavior that
fits an existing action's structure but means something
layer-specific.

```cbor-diag
{
  "type": "fill",
  "section": ".dillo.dtbo",
  "kind": "dillo:dtbo"            ; extension: dillo-specific fill semantics
}
```

`fill` is the canonical example. PMI's own fill kinds (`secrets`,
`cpuid`) name firmware-bound operations the PSP performs; a
`dillo:dtbo` kind names a dillo-defined operation (the VMM
populates the section's GPA range with a host-decided devicetree
overlay per dillo's rules). The PE section and `kind` selector are
PMI's mechanism; the per-kind semantics are the upper layer's spec.

`load` admits the same pattern — a namespaced `kind` defines a new
load variant — though PMI's own load kinds today
(`measured`, `unmeasured`, `vmsa`) cover the firmware-bound cases.

## Companion PE sections

Upper layers commonly carry their data in PE sections named with
the same prefix convention (e.g., `.dillo.dtb`, `.dillo.cfg`). PE
section names are file-format-level — not part of PMI's CBOR
schemas — but the same `<layer>.<...>` convention keeps the layer's
sections recognisable and distinct from PMI-referenced and
image-author-chosen sections.

PMI's only normative role for PE section names is naming the PE
sections used by PMI's own actions and the target-spec sections
(`.pmi.<target>`).

## Example: how a dillo image uses the three points

A `.pmi.sev` spec carrying dillo extensions might look like:

```cbor-diag
{
  "version": 1,
  "id": {"block": ".sev.idb", "auth": ".sev.ida"},
  "actions": [
    {"type": "load", "section": ".linux"},
    {"type": "load", "section": ".dillo.stub"},
    {"type": "load", "section": ".dillo.dtb"},
    {"type": "fill", "section": ".dillo.dtbo", "kind": "dillo:dtbo"},
    {"type": "fill", "section": ".sev.sec",    "kind": "secrets"},
    {"type": "load", "section": ".sev.vms",    "kind": "vmsa"}
  ],
  "dillo:dtb":  ".dillo.dtb",
  "dillo:dtbo": ".dillo.dtbo"
}
```

This image uses all three extension points:

- **Target attributes:** `dillo:dtb` and `dillo:dtbo` tell a
  dillo-aware VMM which PE sections carry the base DTB and the
  dtbo region.
- **Action customization:** the `dillo:dtbo` fill kind tells a
  dillo-aware VMM to populate the section's GPA range per dillo's
  overlay rules (PMI itself has no opinion on the content).
- **New actions:** this example doesn't use any; if dillo needed a
  separate launch-time operation that didn't fit `load` or `fill`,
  it would register a `dillo:<name>` action type.

A pure PMI loader sees the `dillo:*` names, doesn't recognise
them, and refuses to launch — correct, because it can't fulfill
the image's contract. A dillo-aware loader recognises every
namespaced name and processes accordingly.
