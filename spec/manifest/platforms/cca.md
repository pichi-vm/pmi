# Arm CCA Platform Binding

## Platform name

`"cca"` (the key used in the [PMI index](../../index.md)'s `platforms` map).

Convention: `.pmi.cca` PE section for the manifest.

## Segment types

TODO: define launch-input and page-load segment types when the CCA binding is
specified.

## Execution model mapping

| Step          | API call                          |
| ------------- | --------------------------------- |
| 3. Initialize | `RMI_REALM_CREATE` (realm params) |
| 4. Pre-load   | `RMI_RTT_CREATE`                  |
| 5. Segments   | `RMI_DATA_CREATE` per segment     |
| 6. Post-load  | `RMI_REC_CREATE`                  |
| 7. Finalize   | `RMI_REALM_ACTIVATE`              |
