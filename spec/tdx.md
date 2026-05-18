# `tdx` Target

The `tdx` target is the Intel TDX launch path. A VMM targeting `tdx` reads
the `.pmi.tdx` PE section (non-loaded; `IMAGE_SCN_MEM_DISCARDABLE`). If the
section is absent, the image does not support `tdx`.

The `tdx` target is independent of [`vm`](vm.md) and [`sev`](sev.md). It
is expected to reuse the [`load`](load.md) and [`dtbo`](dtbo.md) action
type names with TDX-specific semantics, plus a set of TDX-specific actions
(`tdx:*`) for launch-time inputs.

## Launch model

The `tdx` target follows the [base launch model](vm.md#launch-model)
defined by `vm`, with TDX behavior layered on at the cryptographic steps.

## Status

TODO. The action types, schema, and TDX-specific behavior at each launch
step will be specified here. The current sketch of the underlying TDX
API calls:

| Step          | API                                  |
| ------------- | ------------------------------------ |
| 3. Initialize | `KVM_TDX_INIT_VM` (attributes, xfam) |
| 3. Initialize | `KVM_TDX_INIT_VCPU`                  |
| 4. Update     | `KVM_TDX_INIT_MEM_REGION` per action |
| 4. Update     | TD HOBs                              |
| 5. Finalize   | `KVM_TDX_FINALIZE_VM`                |
