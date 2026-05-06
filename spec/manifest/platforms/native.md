# Native Platform Binding

## Platform key

`"native"`

The native platform is used for non-CC virtual machines. Steps 2–4, 6, and 7
are no-ops. The VMM loads sections (step 5), sets initial register state, and
starts the guest (step 8).

## Policy schema

```cddl
native-policy = {
}
```
