# Extensions

Upper layers (hypervisors, in-guest stubs, image schemas) attach
layer-specific data to PMI images through a single contract: a
namespacing convention for names that appear in the wire format.

## Namespacing

| Class        | Form                | Examples                            | Defined in                                                          |
| ------------ | ------------------- | ----------------------------------- | ------------------------------------------------------------------- |
| Unprefixed   | `name`              | `version`, `actions`, `measured`    | PMI itself (target shape, [`load`](load.md), [`fill`](fill.md))     |
| Registered   | `<layer>:<name>`    | `vm:vcpu`                           | spec linked from the [registry](../README.md#extensions)            |
| Unregistered | `<layer>:<name>`    | layer's choice                      | wherever the layer publishes                                        |

Loaders MUST reject any name they do not understand; a pure PMI
loader sees a prefixed name and refuses to launch, a layer-aware
loader handles the prefixes its spec covers and refuses the rest.

**Registered prefixes** appear in the
[registry](../README.md#extensions). To register, open a PR
against the PMI spec repository with the prefix and a link to its
spec.

**Unregistered prefixes** are not coordinated with PMI; the layer
chooses a collision-resistant prefix (e.g., derived from a domain
it controls, a UUID) and publishes its own spec. Suited for
private, experimental, or deployer-specific layers.

## Four extension points

Point 1 is registered-only; points 2–4 are open to both classes.

### 1. New targets (registered only)

A new launch target — a `.pmi.<target>` PE section with its own
schema and launch model. A target's CBOR map MUST follow this
skeleton; the target extends it through the other three extension
points.

```cddl
target = { "version" => uint, "actions" => [+ action] }
action = { "type" => tstr, * tstr => any }
```

`.pmi.` is PMI's PE-section namespace, hence registered-only.

### 2. Target attributes (top-level keys)

A namespaced top-level CBOR key carries metadata the extension
needs at launch, independent of any action. The value is any CBOR
type the extension specifies.

### 3. New actions

A namespaced action `type` adds a new operation to the actions
array. Beyond `type`, the action's shape is entirely the
extension's spec — fields, runtime behavior, firmware or VMM calls,
measurement contribution.

### 4. Action-defined extension points

An action's own schema MAY declare an extension point. PMI's
[`load`](load.md) and [`fill`](fill.md) declare their `kind` field
as a free-form text string, admitting namespaced kinds alongside
the per-target kinds the target chapters enumerate. Future actions
decide their own extension point (or none).
