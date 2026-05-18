# `cca` Target

The `cca` target is the Arm CCA launch path. A VMM targeting `cca` reads the
`.pmi.cca` PE section (non-loaded; `IMAGE_SCN_MEM_DISCARDABLE`). If the
section is absent, the image does not support `cca`.

The `cca` target is independent of [`vm`](vm.md) and [`sev`](sev.md). It
is expected to reuse the [`load`](vm.md#load-action) and
[`dtbo`](vm.md#dtbo-action) action type names with CCA-specific
semantics, plus a set of CCA-specific actions (`cca:*`) for realm
creation and activation.

## Launch model

The `cca` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with CCA behavior layered on at the cryptographic steps.

## Status

TODO. The action types, schema, and CCA-specific behavior at each launch
step will be specified here. The current sketch of the underlying CCA
API calls:

| Step          | API                               |
| ------------- | --------------------------------- |
| 3. Initialize | `RMI_REALM_CREATE` (realm params) |
| 3. Initialize | `RMI_RTT_CREATE`                  |
| 4. Update     | `RMI_DATA_CREATE` per action      |
| 4. Update     | `RMI_REC_CREATE`                  |
| 5. Finalize   | `RMI_REALM_ACTIVATE`              |
