# Arm CCA Platform Binding

## Platform key

`"cca"`

## Policy schema

```cddl
; TODO: define when CCA binding is specified.
cca-policy = {
}
```

## Execution model mapping

| Step          | API call                          |
| ------------- | --------------------------------- |
| 3. Initialize | `RMI_REALM_CREATE` (realm params) |
| 4. Pre-load   | `RMI_RTT_CREATE`                  |
| 5. Segments   | `RMI_DATA_CREATE` per segment     |
| 6. Post-load  | `RMI_REC_CREATE`                  |
| 7. Finalize   | `RMI_REALM_ACTIVATE`              |
