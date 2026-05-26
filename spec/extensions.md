# Extensions

The PMI core specification does not provide sufficient functionality to launch a
guest without defined extensions. This means that most of the actual
functionality is found in PMI extensions.

PMI provides two classes of extensions:

1. **Registered** extensions appear in the
   [registry](../README.md#extension-registry). To register, open a PR against
   the PMI spec repository.

2. **Unregistered** are not coordinated with PMI; an application which wants to
   use an unregistered extension chooses a collision-resistant prefix and
   publishes its own spec. Suited for private, experimental, or
   deployer-specific layers.

## Namespacing

PMI extensions follow namespacing rules when used as values or map keys. Only
the PMI core specification can use unprefixed names. Extensions MUST use an
extension-defined prefix: either the registered prefix name or a
collision-resistant name (when used in an unregistered extension).

| Class        | Form              | Examples              | Defined in                                                       |
| ------------ | ----------------- | --------------------- | ---------------------------------------------------------------- |
| Unprefixed   | `name`            | `version`, `actions`  | PMI core specification                                           |
| Registered   | `<prefix>:<name>` | `vm:vcpu`             | spec linked from the [registry](../README.md#extension-registry) |
| Unregistered | `<prefix>:<name>` | `com.foo.bar:my-data` | wherever the extension publishes                                 |

An unknown map key always causes the launch to fail. PMI decodes every CBOR map
in strict mode — there are no ignored or pass-through keys — so an unrecognized
name surfaces as an eager, explicit error rather than a subtly misconfigured VM.

## Four extension points

PMI can be extended in four different ways.

### 1. New targets (registered only)

A registered prefix MAY define a new launch target — a `.pmi.<prefix>` PE
section carrying a CBOR spec that follows the
[common target shape](core.md#shape). A new target MUST define the accepted
`version` value(s). PE section names starting with `.pmi.` are PMI's namespace.
Therefore, new targets may only be defined by registered extensions.

### 2. Target attributes (top-level keys)

An extension MAY define a new **target** attribute. This allows the inclusion of
top-level metadata for a target. The target attribute name MUST follow the
namespacing rules and its value MUST be valid CBOR.

### 3. New action types

An extension MAY define a new **action** `type`. This permits extensions to
define new actions (beside `load` and `fill`) for use during VM construction.
The **action** `type` values MUST follow the namespacing rules.

### 4. Action-defined extension points

An action's own schema MAY declare an extension point. PMI's
[`load`](core.md#load) and [`fill`](core.md#fill) declare their `kind` field as
such an extension point. Extension points defined by an **action** definition
MUST define how namespacing rules are applicable.
