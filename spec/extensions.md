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
classes feed into the same three extension points on a target spec.

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

- **Unprefixed names** (e.g., `version`, `actions`, `secrets`,
  `cpuid`) are reserved for PMI itself.
- **Registered prefixed names** of the form `<layer>:<name>`,
  where `<layer>` appears in the [Extension registry](#extension-registry)
  below.
- **Unregistered prefixed names** of the form `<layer>:<name>`,
  where `<layer>` is a collision-resistant reverse-DNS identifier.

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

A registered `<layer>` prefix:

- Is a single segment with no dots.
- Appears in the [Extension registry](#extension-registry) below,
  which points at the layer's authoritative spec.
- Is short and memorable — the registry exists so registered
  prefixes don't collide and so any loader can find the spec for a
  prefix it encounters.

A layer becomes registered by opening an issue or pull request
against the PMI spec repository with the proposed prefix and a
link to its spec. Once accepted, the prefix appears in the
registry and is considered part of PMI's stable extension
surface.

### Unregistered extensions

An unregistered `<layer>` prefix:

- MUST contain at least one dot.
- SHOULD use reverse-DNS form
  (`com.example.foo`, `org.openstack.bar`) under a domain the
  layer controls, or a similar collision-resistant scheme
  (`urn.uuid.<uuid>`).
- Has no entry in the registry; the layer is responsible for
  documenting its own contract however it sees fit.

Unregistered extensions exist for layers that are private,
experimental, deployer-specific, or simply not yet ready to
register. They are first-class — a layer-aware loader honors them
exactly like registered ones — but PMI does not vouch for them and
provides no discoverability beyond what the layer publishes
itself.

### Distinguishing the two classes

The dot/no-dot rule makes the two classes syntactically
distinguishable without consulting the registry: any prefix
containing a dot is unregistered; any prefix without a dot is
registered (or invalid, if it's not in the registry).

This lets a loader route prefix resolution efficiently — checking
the local registry for dotless prefixes, falling back to its
configured set of supported reverse-DNS prefixes for dotted ones —
but functionally both classes are handled identically: recognised
names are honored, unrecognised names cause launch refusal.

## Extension registry

The following prefixes are registered with PMI. Each entry links
to the layer's authoritative spec.

| Prefix | Spec |
| ------ | ---- |
| _(none yet)_ | |

To register a prefix, open an issue or pull request against the
PMI spec repository with the proposed prefix and a link to the
layer's spec.

## Three extension points

Either class of prefix attaches behavior to a target spec at one
of three places.

### 1. Target attributes (top-level keys)

A namespaced top-level CBOR key adds a piece of metadata the upper
layer needs to know about, independent of any action.

```cbor-diag
{
  "version": 1,
  "actions": [ ... ],

  "registered:platform":    <layer-defined value>,
  "com.example.bar:config": <layer-defined value>
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
  "type": "com.example.bar:provision"
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
  "kind": "registered:<name>"
}
```

```cbor-diag
{
  "type": "fill",
  ...,
  "kind": "com.example.bar:<name>"
}
```

`fill` is the canonical example: its `kind` field selects between
firmware-bound operations PMI defines (`secrets`, `cpuid`) and
namespaced kinds upper layers register or define out-of-band. The
`kind` selector is PMI's mechanism; the per-kind semantics are the
upper layer's spec.

`load` admits the same pattern, though PMI's own load kinds today
(`measured`, `unmeasured`, `vmsa`) cover the firmware-bound cases.
