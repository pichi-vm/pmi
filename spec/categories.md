# Categories (non-normative design rationale)

> **Non-normative.** This document was developed alongside PMI to
> understand how data flows through a confidential VM launch — from
> image, through firmware setup, into attestation. The category
> framework is normative for **upper-layer specs** (e.g., dillo) that
> define platform identity, attestation policy, host-conformance
> rules, and the carriage of host-supplied platform information. PMI
> itself carries firmware-bound launch mechanisms only; this analysis
> informs PMI's [layering](overview.md#extensions) but does not
> constrain PMI's wire format.

## Why PMI's scope is what it is

The PMI layering — firmware-bound mechanics in PMI, platform
semantics in an upper layer — comes from tracing each kind of data
through three stages: where it originates (image, deployer, host,
allocator), where it lands in firmware setup (what API consumes it),
and where it shows up in attestation (in the cryptographic
measurement, in a separate report field, or nowhere). Data that the
firmware ABIs themselves consume stays in PMI; everything else moves
up.

## The categories

| Category               | What it is                                                                                                       | Source                                                       | Measured? | In attestation report?      |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | --------- | --------------------------- |
| **Image identity**     | The workload bytes — kernel, initrd, command line, firmware, in-guest stubs                                      | PMI image                                                    | Yes       | Yes (in measurement)        |
| **Platform identity**  | The hardware *shape* the workload expects — devices, MMIO, IRQ controller, PCIe topology, CC-feature requirements | Upper-layer image declarations                               | Yes       | Yes (in measurement)        |
| **Tenant identity**    | A hash or signature binding a deployment to a tenant — SEV id-block/id-auth, TDX MR\*, CCA RPV                   | PMI image (when tenant is the image author) or runtime input | No        | Yes (separate report field) |
| **Host identity**      | Host-supplied attestation data naming a host operator — e.g., SEV `HOST_DATA`                                    | Runtime input                                                | No        | Yes (separate report field) |
| **Deployer policy**    | Operational metadata the verifier checks against policy — SEV-SNP POLICY when no ID block is present             | Runtime input                                                | No        | Yes (separate report field) |
| **Instance accidents** | Per-launch sizing and wiring with no identity meaning — vCPU count, memory size, NUMA, aux granules, EPTP        | Runtime input                                                | No        | No                          |

Image identity is in PMI today: PMI's measured `load` action is how
image bytes reach the launch measurement. Tenant identity is in PMI
when image-bound (SEV's `id` block lives in `.pmi.sev`). Host
identity, deployer policy, and instance accidents are channels PMI
doesn't carry — vendor APIs deliver them to firmware directly.

Platform identity is the interesting one: it is what an upper layer
(dillo) defines, declares, and binds. The upper layer carries its
own DTB and any platform descriptors via PMI's load and fill actions
plus the [Extensions](overview.md#extensions) namespace.

## Topological mapping (the trace that informed the split)

Tracing every parameter PMI and the vendor APIs touch yields five
flow topologies based on where the value ends up in attestation:

| Topology | Where the value lands in attestation                          | Cryptographic binding             | Examples                                                          | Layer                                                       |
| -------- | -------------------------------------------------------------- | --------------------------------- | ----------------------------------------------------------------- | ----------------------------------------------------------- |
| **A**    | In the cryptographic measurement register                      | The measurement itself            | Image bytes via `load`; TDX `ATTRIBUTES`, `XFAM`, `CPUID_CONFIG`  | PMI (loads) + upper layer (promotion of vendor-API fields) |
| **B**    | Separate attestation field; carried in a signed structure      | Tenant key (via the signature)    | SEV POLICY when wrapped by an `id` block                          | PMI carries the signed structure (e.g., `id_block`)         |
| **C**    | Separate attestation field; not signed                         | None — verifier-policy check only | SEV POLICY when no ID block                                       | Vendor channel; PMI does not carry                          |
| **D**    | Typed-page measurement (GPA + page type bound, content unbound) | Page type and placement only      | SEV `cpuid` / `secrets` page contents                             | PMI carries the fill action; content semantics per vendor   |
| **E**    | Not in the attestation report at all                           | None                              | Resource allocation, allocator output                             | Upper layer (instance accidents)                            |

Observations from the trace:

1. **The same bits can change topology between launches.** SEV POLICY
   sits in topology C without an ID block and shifts to topology B
   when the deployer wraps it in a signed ID block. The bits don't
   change; their cryptographic binding does.

2. **Measured leftover (topology A) demands image-bound declaration.**
   Every bit in the cryptographic measurement is part of the image's
   identity by construction. If the value is host-decided, two
   conformant VMMs of the same image diverge measurement — breaking
   attestation equivalence. Upper layers close this gap by declaring
   the expected bytes in a measured load that the VMM is required to
   pass through to the firmware API verbatim.

## Promotion via measured load

For any value that ought to be image-bound but currently isn't —
vendor-API fields the firmware measures (TDX `ATTRIBUTES`/`XFAM`,
CCA `RmiRealmParams`), the SEV-SNP CPUID page content, etc. — the
upper layer "promotes" the value to image identity by:

1. Declaring the bytes in a PE section the image owns
2. Using PMI's measured `load` action (or a future measured fill kind
   with vendor-API binding semantics) to bind the bytes to the launch
   measurement
3. Requiring the VMM to submit those exact bytes to the firmware API
   that consumes the field

A conformant VMM passes the declared bytes through unchanged. A VMM
that substitutes a different value diverges the measurement and the
verifier rejects.

This is the path upper layers take to close gaps vendor designs left
open for evolutionary reasons. PMI provides the mechanism (measured
load, namespaced extension kinds); the upper layer defines which
fields to promote and exactly what the VMM must do with them.
