# Extensions

PMI is a substrate. The PE container, the per-target CBOR launch
recipes, and the action mechanism cover the firmware-bound mechanics
of every launch — and stop there. Upper layers (hypervisor specs,
in-guest stubs, image schemas) need to attach layer-specific data
to PMI images and have a VMM that understands them do something
with it.

PMI exposes a single extensibility contract: a namespacing
convention that admits two classes of extension — **registered**
extensions defined within the PMI spec, and **unregistered**
extensions any layer can use without coordinating with PMI. Both
classes feed into a set of four extension points; one of those is
registered-only.

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

## Namespacing

Names that appear in the wire format fall into one of three
classes:

- **Unprefixed names** (e.g., `version`, `actions`, `measured`)
  are reserved for PMI itself — the spec defines them in its
  core pages (target shape, `load`, `fill`).
- **Registered prefixed names** of the form `<layer>:<name>`,
  where `<layer>` appears in the [Extension registry](#extension-registry)
  below.
- **Unregistered prefixed names** of the form `<layer>:<name>`,
  where `<layer>` is a collision-resistant identifier chosen by
  the layer.

The prefix names the **consumer** — typically a hypervisor or
in-guest stub — not the producer. Multiple image tools may emit
the same prefix for the same consumer.

Loaders MUST reject any name they do not understand. A pure PMI
loader presented with any prefixed name refuses to launch; a
layer-aware loader handles only the prefixes its spec covers and
refuses the rest. The strict-rejection rule already governing
unknown PMI keys, types, and kinds handles unknown namespaced
names with no new mechanism.

### Registered extensions

A registered `<layer>` prefix appears in the
[Extension registry](#extension-registry) below, which points at
the layer's authoritative spec. The registry exists so registered
prefixes don't collide and so any loader can find the spec for a
prefix it encounters.

A layer becomes registered by opening an issue or pull request
against the PMI spec repository with the proposed prefix and a
link to its spec. Once accepted, the prefix appears in the
registry and is considered part of PMI's stable extension
surface.

### Unregistered extensions

An unregistered `<layer>` prefix has no entry in the registry. The
layer chooses the prefix and is responsible for ensuring it is
collision-resistant — there is no central coordinator preventing
two unregistered layers from picking the same prefix, so the
prefix must be unique by construction (e.g., derived from a domain
the layer controls, a UUID, or any other scheme that makes
accidental collision negligible).

Unregistered extensions exist for layers that are private,
experimental, deployer-specific, or simply not yet ready to
register. They are first-class — a layer-aware loader honors them
exactly like registered ones — but PMI does not vouch for them and
provides no discoverability beyond what the layer publishes
itself.

## Extension registry

The following prefixes are registered with PMI. Each entry links
to the layer's authoritative spec.

| Prefix  | Spec                          |
| ------- | ----------------------------- |
| `vm`    | [spec/vm.md](vm.md) target    |
| `sev`   | [spec/sev.md](sev.md) target  |
| `cca`   | [spec/cca.md](cca.md) target  |
| `tdx`   | [spec/tdx.md](tdx.md) target  |

The four current targets are themselves registered extensions —
each one owns the `<target>` name in the registry and the
corresponding `.pmi.<target>` PE section (see the
[target extension point](#4-new-targets-registered-only) below).

To register a prefix, open an issue or pull request against the
PMI spec repository with the proposed prefix and a link to the
layer's spec.

## Four extension points

A prefix attaches behavior to a PMI image at one of four places.
Points 1–3 work for both registered and unregistered prefixes;
point 4 is reserved for registered prefixes only.

### 1. Target attributes (top-level keys)

A namespaced top-level CBOR key adds a piece of metadata the upper
layer needs to know about, independent of any action.

```cbor-diag
{
  "version": 1,
  "actions": [ ... ],

  "registered:platform":    <layer-defined value>,
  "unregistered:config": <layer-defined value>
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
  "type": "registered:configure"
  ; per-action fields the upper layer defines
}
```

```cbor-diag
{
  "type": "unregistered:provision"
  ; per-action fields the upper layer defines
}
```

PMI executes actions in array order; an upper-layer action runs at
its array position relative to PMI's actions. Beyond `type`, the
shape of an upper-layer action is entirely the upper layer's spec:
it chooses its own fields and defines what the action does — what
the VMM submits, what firmware or VMM calls happen, what
measurement (if any) the operation contributes to.

### 3. Action-defined extension points

An individual action's schema may declare its own extension point.
This is not a generic PMI mechanism — it is a property of specific
actions whose specs opt into it. The shape of the extension point
is whatever that action's spec defines.

PMI's two built-in actions — [`load`](load.md) and
[`fill`](fill.md) — both declare their `kind` field as a
free-form text string, explicitly admitting namespaced values
alongside the per-target kinds the target chapters enumerate:

```cbor-diag
{
  "type": "load",
  ...,
  "kind": "registered:<name>"
}
```

```cbor-diag
{
  "type": "fill",
  ...,
  "kind": "unregistered:<name>"
}
```

The `kind` selector and the per-kind semantics are the action's
contract; the namespacing rule is what lets upper layers
participate without colliding.

Future actions MAY define their own extension points (the same
`kind`-style pattern, or something entirely different), or none at
all. The spec defining the action decides.

### 4. New targets (registered only)

A registered prefix MAY define a new launch target — a new
`.pmi.<target>` PE section whose schema and launch model the
registered spec defines.

```
.pmi.<registered>      ; e.g., .pmi.dillo for a hypothetical
                       ; registered extension named `dillo`
```

This extension point is reserved for registered prefixes. PE
section names starting with `.pmi.` are PMI's namespace; allowing
unregistered prefixes to claim names there would conflict with
the rule that loaders refuse images they don't understand and
muddle the discovery model (loaders look for `.pmi.<target>`
sections by name).

To define a new target, register the prefix per
[Extension registry](#extension-registry) and have the spec
follow the [common target shape](#common-target-shape): a
CBOR map with `version` and `actions`, plus whatever
per-target firmware-bound fields the new target needs.
