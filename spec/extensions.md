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

## Namespacing

Names that appear in the wire format fall into one of three
classes:

- **Unprefixed names** (e.g., `version`, `actions`, `measured`)
  are reserved for PMI itself — the spec defines them in its
  core pages (target shape, `load`, `fill`).
- **Registered prefixed names** of the form `<layer>:<name>`,
  where `<layer>` appears in the
  [extension registry](../README.md#extensions) at the bottom of
  the project README.
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
[extension registry](../README.md#extensions) (bottom of the
project README), which points at the layer's authoritative spec.
The registry exists so registered prefixes don't collide and so
any loader can find the spec for a prefix it encounters.

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

## Four extension points

A prefix attaches behavior to a PMI image at one of four places.
Point 1 is reserved for registered prefixes only; points 2–4 work
for both registered and unregistered prefixes.

### 1. New targets (registered only)

A registered prefix MAY define a new launch target — a new
`.pmi.<target>` PE section whose schema and launch model the
registered spec defines. This extension point is reserved for
registered prefixes. PE section names starting with `.pmi.` are
PMI's namespace; allowing unregistered prefixes to claim names
there would conflict with the rule that loaders refuse images
they don't understand and muddle the discovery model (loaders
look for `.pmi.<target>` sections by name).

A target's CBOR map follows this skeleton:

```cddl
target = {
  "version" => uint,                       ; schema version
  "actions" => [+ action],                 ; ordered launch recipe
  ; per-target firmware-bound fields (extension point 2)
}

action = {
  "type" => tstr,                          ; selects load / fill / ...
  ; per-type fields
}
```

`type` is the only universal action field. Everything else is
defined per action type.

To define a new target, register the prefix per the
[extension registry](../README.md#extensions) and have the spec
specify what target attributes, action types, and action kinds
the target adds to this skeleton.

### 2. Target attributes (top-level keys)

A namespaced top-level CBOR key adds a piece of metadata the
extension needs at launch, independent of any action. The value
can be any CBOR type the extension specifies. PMI ignores
everything under a prefix it doesn't own; a layer-aware loader
reads its own `<layer>:*` keys per the layer's spec.

### 3. New actions

A namespaced `type` value adds a new kind of operation the
extension wants performed at launch, alongside PMI's own actions.

PMI executes actions in array order; an extension action runs at
its array position relative to PMI's actions. Beyond `type`, the
shape of an extension action is entirely the extension's spec —
fields, what the action does, what firmware or VMM calls happen,
what measurement (if any) the operation contributes to.

### 4. Action-defined extension points

An individual action's schema may declare its own extension
point. This is not a generic PMI mechanism — it is a property of
specific actions whose specs opt into it. The shape of the
extension point is whatever that action's spec defines.

PMI's two built-in actions — [`load`](load.md) and
[`fill`](fill.md) — both declare their `kind` field as a
free-form text string, explicitly admitting namespaced values
alongside the per-target kinds the target chapters enumerate. The
`kind` selector and the per-kind semantics are the action's
contract; the namespacing rule is what lets extensions
participate without colliding.

Future actions MAY define their own extension points (the same
`kind`-style pattern, or something entirely different), or none
at all. The spec defining the action decides.
