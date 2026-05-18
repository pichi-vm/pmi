# `tdx` Target

The `tdx` target is the Intel TDX launch path. A VMM targeting `tdx` reads
the `.pmi.tdx` PE section (non-loaded; `IMAGE_SCN_MEM_DISCARDABLE`). If the
section is absent, the image does not support `tdx`.

The `tdx` target is independent of [`vm`](vm.md) and [`sev`](sev.md). It is
expected to share the `dtb` field and the [`load`](load.md) and
[`dtbo`](dtbo.md) actions, plus a set of TDX-specific actions (`tdx:*`) for
launch-time inputs.

## Status

TODO. The action types, schema, and execution-model mapping for `tdx` will be
specified here. The current sketch of the underlying TDX API calls:

| Step          | API                                  |
| ------------- | ------------------------------------ |
| 4. Initialize | `KVM_TDX_INIT_VM` (attributes, xfam) |
| 4. Initialize | `KVM_TDX_INIT_VCPU`                  |
| 6. Update     | `KVM_TDX_INIT_MEM_REGION` per action |
| 6. Update     | TD HOBs                              |
| 8. Finalize   | `KVM_TDX_FINALIZE_VM`                |
