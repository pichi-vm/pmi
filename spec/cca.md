# `cca` Target

The `cca` target is the Arm CCA launch path. A VMM targeting `cca` reads the
`.pmi.cca` PE section (non-loaded; `IMAGE_SCN_MEM_DISCARDABLE`). If the
section is absent, the image does not support `cca`.

The `cca` target is independent of [`vm`](vm.md) and [`sev`](sev.md). It
is expected to reuse the [`load`](load.md) and [`dtbo`](dtbo.md) action
type names with CCA-specific semantics, plus a set of CCA-specific actions
(`cca:*`) for realm creation and activation.

## Status

TODO. The action types, schema, and execution-model mapping for `cca` will be
specified here. The current sketch of the underlying CCA API calls:

| Step          | API                               |
| ------------- | --------------------------------- |
| 4. Initialize | `RMI_REALM_CREATE` (realm params) |
| 4. Initialize | `RMI_RTT_CREATE`                  |
| 6. Update     | `RMI_DATA_CREATE` per action      |
| 6. Update     | `RMI_REC_CREATE`                  |
| 8. Finalize   | `RMI_REALM_ACTIVATE`              |
