# Intel TDX Platform Binding

## Platform name

`"tdx"` (the key used in the [PMI index](../../index.md)'s `platforms` map).

The PE section containing this platform's manifest may use any name; only the
index is authoritative. By convention images use `.pmi.tdx`.

## Segment types

TODO: define launch-input and page-load segment types when the TDX binding is
specified.

## Execution model mapping

| Step          | API call                              |
| ------------- | ------------------------------------- |
| 3. Initialize | `KVM_TDX_INIT_VM` (attributes, xfam)  |
| 4. Pre-load   | `KVM_TDX_INIT_VCPU`                   |
| 5. Segments   | `KVM_TDX_INIT_MEM_REGION` per segment |
| 6. Post-load  | TD HOBs                               |
| 7. Finalize   | `KVM_TDX_FINALIZE_VM`                 |
