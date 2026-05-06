# Intel TDX Platform Binding

## Platform key

`"tdx"`

## Policy schema

```cddl
; TODO: define when TDX binding is specified.
tdx-policy = {
}
```

## Execution model mapping

| Step          | API call                               |
| ------------- | -------------------------------------- |
| 3. Initialize | `KVM_TDX_INIT_VM` (attributes, xfam)  |
| 4. Pre-load   | `KVM_TDX_INIT_VCPU`                   |
| 5. Sections   | `KVM_TDX_INIT_MEM_REGION` per section  |
| 6. Post-load  | TD HOBs                                |
| 7. Finalize   | `KVM_TDX_FINALIZE_VM`                  |
