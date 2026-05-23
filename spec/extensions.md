# Extensions

PMI is a substrate. The PE container, the per-target CBOR launch
recipes, and the action mechanism cover the firmware-bound mechanics
of every launch — and stop there. Upper layers (hypervisor specs,
in-guest stubs, image schemas) need to attach layer-specific data to
PMI images and have a VMM that understands them do something with
it. PMI exposes a single extensibility contract — a namespacing
convention — and three extension points that bolt new behavior onto
a target spec without changing PMI's core shape.

## Common target shape

Every PMI target spec is a CBOR map with this skeleton:

```cddl
target = {
  "version" => uint,                       ; schema version
  "actions" => [+ action],                 ; ordered launch recipe
  ; per-target firmware-bound fields (e.g., sev's `id`, vm/cca's `vcpu`)
}

action = {
  "type" => tstr,                          ; selects load / fill / ...
  ; per-type fields
}
```

`type` is the only universal action field. Everything else is
defined per action type. A new action a layer registers chooses its
own fields independent of what PMI's existing actions use.

Across `vm`, `sev`, `cca`, and `tdx` the shape never changes; only
the per-target firmware-bound side fields, the action types each
target admits, and the ABI those actions drive change. An upper
layer extends this shape without re-specifying it.

## Namespacing rule

PMI's only extensibility primitive: a namespacing convention for
names that appear in the wire format.

- **Unprefixed names** (e.g., `version`, `actions`, `secrets`,
  `cpuid`) are reserved for PMI.
- **Prefixed names** of the form `<layer>:<name>` belong to the
  named upper layer.
- The prefix names the **consumer** — typically a hypervisor or
  in-guest stub — not the producer. Multiple image tools may emit
  the same prefix for the same consumer.
- Loaders MUST reject any name they do not understand. A pure PMI
  loader presented with any `<layer>:*` name refuses to launch; a
  layer-aware loader handles it per the layer's spec.

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
  "<layer>:platform":  <layer-defined value>,
  "<layer>:something": <layer-defined value>
}
```

The value can be any CBOR type the upper layer specifies. PMI
ignores everything under a prefix it doesn't own; a layer-aware
loader reads its own `<layer>:*` keys per the layer's spec.

### 2. New actions

A namespaced `type` value adds a new kind of operation the upper
layer wants performed at launch, alongside PMI's own actions.

```cbor-diag
{
  "type": "<layer>:configure",
  ; per-action fields the upper layer defines
}
```

PMI executes actions in array order; an upper-layer action runs at
its array position relative to PMI's actions. Beyond `type`, the
shape of an upper-layer action is entirely the upper layer's spec:
it chooses its own fields and defines what the action does — what
the VMM submits, what firmware or VMM calls happen, what
measurement (if any) the operation contributes to.

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
  ...,
  "kind": "<layer>:<name>"
}
```

`fill` is the canonical example: its `kind` field selects between
firmware-bound operations PMI defines (`secrets`, `cpuid`) and
namespaced kinds upper layers register. The `kind` selector is
PMI's mechanism; the per-kind semantics are the upper layer's spec.

`load` admits the same pattern, though PMI's own load kinds today
(`measured`, `unmeasured`, `vmsa`) cover the firmware-bound cases.
