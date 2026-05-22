# Categories

PMI distinguishes five categories of data that flow through any
launch: four **identity** categories (image, platform, tenant, host)
and one **non-identity** category (instance accidents). The categories
give the spec a uniform vocabulary for talking about what gets
measured, what reaches the attestation report through side channels,
and what has no identity meaning at all.

The categories are PMI's conceptual model for confidential computing
as a whole — a unified taxonomy meant to apply across SEV-SNP, TDX,
CCA, and any future CC target. The individual vendor designs are
older than PMI and evolved independently; they will not all map
cleanly onto the model. Per-target chapters sort each parameter into
the model where it fits and surface the rest as **leftover** so the
mismatches are visible. Leftovers may indicate vendor-specific
evolutionary quirks, or they may indicate gaps in PMI's model that
need to grow; classifying which is which is open work
([Leftover values](#leftover-values)).

This page defines each category in depth, gives a decision procedure
for classifying any new target-specific parameter, and discusses how
to think about leftover values. Each per-target chapter
([`sev`](sev.md), [`cca`](cca.md), [`tdx`](tdx.md)) enumerates its
own parameters against this framework. [`vm`](vm.md) has no
attestation channel, so the identity categories do not apply to it
directly.

For the summary table and the goals these categories serve, see
[Overview](overview.md#categories).

## The five categories

### Image identity

The workload bytes themselves: kernel, initrd, command line, firmware
or in-guest stubs, any other content loaded into guest memory at
launch by a `load` action with a measurement-bearing kind. These are
the bytes of the workload the deployer wanted to run.

- **Source.** The PMI image. PE sections referenced by `load` actions
  carry the bytes; the action's kind selects the measurement path.
- **Measurement.** Contributes to the cryptographic launch
  measurement (SEV-SNP launch digest, CCA RIM, TDX MRTD) through the
  firmware path the target binds to its measured `load` kind.
- **Attestation report.** Present in the measurement register; not
  surfaced separately.
- **Who decides.** The image author.
- **What changes it.** Changing the workload — rebuilding the kernel,
  adding an initrd, editing the command line.

The defining property: image identity is the **contents** the
workload runs. Two images with identical platform declarations but
different kernels are different workloads.

### Platform identity

The fundamental hardware contract the workload expects from the
platform: which devices exist, where MMIO regions live, which
interrupt controller version, what PCIe topology, what CPU mode the
boot vCPU enters in, which CC extensions must be active. This is
*shape*, not *size*.

- **Source.** The PMI image. Concretely: the [base DTB](dtb.md)
  ([`dtb`](vm.md#schema) field), the [`vcpu`](vm.md#vcpu-field)
  register map (or the BSP REC parameters on CCA, or the
  reset-vector PMI consumer on TDX), and a target-specific set of
  liveness-requirement bits within vendor structures (e.g., specific
  bits of TDX `ATTRIBUTES` and `XFAM` that name CC features the image
  needs to run).
- **Measurement.** Contributes to the cryptographic launch
  measurement, either directly (vendor structures measured by
  firmware, e.g., TDX `TD_PARAMS`) or because the image-side stub
  that consumes it is itself measured (the base DTB is part of the
  image bytes; the in-guest consumer that merges DTB+dtbo is
  measured).
- **Attestation report.** Present in the measurement register.
- **Who decides.** The image author.
- **What changes it.** Changing the *shape* of the hardware
  contract — adding a device, moving an MMIO region, requiring a
  different interrupt controller version, requiring a different CC
  feature (LPA2 on, SEPT_VE_DISABLE on).

The defining property: platform identity is the **shape** the
workload expects. The 4-vCPU and 8-vCPU deployments of the same
image have identical platform identity — the sizing is instance
accident, not shape change. Changing MMIO addresses or interrupt
controller version is a different shape.

### Tenant identity

A hash, signature, or other deployer-supplied value that binds a
deployment to a particular tenant. Concretely: SEV-SNP's signed
[`id` block](sev.md#id-field) (and the ID auth info that signs it),
CCA's Realm Personalization Value (RPV), TDX's `MRCONFIGID`,
`MROWNER`, and `MROWNERCONFIG` fields.

- **Source.** The PMI image when the image author is the tenant
  (SEV's `id` block is carried in PMI as PE sections); runtime input
  otherwise (CCA's RPV and TDX's MR\* fields are passed via VMM
  config, not PMI).
- **Measurement.** Does not enter the cryptographic launch
  measurement.
- **Attestation report.** Surfaced through separate firmware
  channels — `SNP_LAUNCH_FINISH` for SEV's ID block, the Realm Token
  for CCA's RPV, TDREPORT fields for TDX's MR\* values.
- **Who decides.** The tenant (who may or may not be the image
  author).
- **What changes it.** A different tenant deploying the same image;
  a tenant re-signing the same image with a different key.

The defining property: tenant identity says **who owns** this
deployment, not what runs. Two tenants deploying the same image to
the same hardware shape produce identical image+platform identity
and divergent tenant identity.

### Host identity

Host-supplied attestation data: bytes the deployer passes into the
firmware that surface in the attestation report but bind to neither
the image nor the tenant. SEV-SNP's `HOST_DATA` is the canonical
example.

- **Source.** Runtime input. PMI never declares host identity.
- **Measurement.** Does not enter the cryptographic launch
  measurement.
- **Attestation report.** Surfaced through a separate firmware
  channel.
- **Who decides.** The host operator.
- **What changes it.** The host passing different `HOST_DATA` on
  the next launch.

The defining property: host identity says **which host
deployment** ran this image, for verifier policies that want to
correlate a launch to operator-side state (a specific control
plane, a specific orchestrator run, a specific tenant binding the
host made elsewhere). It is host-supplied and PMI's role is solely
to acknowledge that the channel exists.

### Instance accidents

Per-launch sizing, wiring, and allocator output that has no identity
meaning. Concretely: vCPU count and memory size and NUMA topology
(carried in the [`dtbo`](vm.md#dtbo-overlay)); CCA auxiliary REC
granule addresses; TDX EPTP controls; any VMM-internal scheduling,
locality, or allocation choice that doesn't perturb what the guest
sees as its hardware contract.

- **Source.** Runtime input. The host decides them per launch.
- **Measurement.** Does not enter the cryptographic launch
  measurement.
- **Attestation report.** Does not appear in the attestation report
  at all.
- **Who decides.** The host operator, per launch, with no binding
  to the image or the tenant.
- **What changes it.** Scaling the deployment, rebalancing NUMA,
  the host's allocator picking different addresses.

The defining property: instance accidents are values the verifier
**does not need to know** to reproduce the expected attestation. The
4-vCPU and 8-vCPU deployments of the same image must produce the
same launch measurement — instance accidents are exactly the
parameters where this guarantee bites.

Many parameters that look like they ought to be host-decided
configuration are in fact platform identity: changing them
**would** change the workload's contract. The test is whether the
guest can observe the change as a different hardware shape. vCPU
count is observable but doesn't change shape (just size); MMIO
location is observable and changes shape. The former is instance
accident; the latter is platform identity.

## Decision procedure

For any new target-specific parameter, walk these questions in order
and assign the first match:

1. **Are the bytes themselves part of the workload?** (Kernel,
   initrd, command line, in-guest stub.) → **image identity**.

2. **Does the workload depend on this value being what it is to
   run correctly?** (A required CC feature, an MMIO layout the
   kernel was compiled against, a CPU mode the boot stub expects.)
   → **platform identity**.

3. **Does this value name a tenant or sign a deployment?**
   (A signed identity block, a deployer-supplied RPV, an
   `MROWNER` value.) → **tenant identity**.

4. **Is this host-supplied data that surfaces in the attestation
   report but doesn't bind the image or the tenant?**
   (`HOST_DATA`.) → **host identity**.

5. **Is this per-launch sizing, allocator output, or VMM-internal
   configuration that doesn't change what the guest sees as its
   hardware shape?** (vCPU count, memory size, aux granule
   addresses, EPTP controls.) → **instance accident**.

If none of the five questions match, the parameter is **leftover**;
see [Leftover values](#leftover-values) below.

If a single vendor field's bits answer differently across the
questions, split the field by bit and classify each bit
independently.

## Where each category lives in PMI

| Category           | In PMI?                                  | Concrete mechanism                                                                                |
| ------------------ | ---------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Image identity     | Yes                                      | PE sections referenced by measured `load` actions                                                 |
| Platform identity  | Yes                                      | [Base DTB](dtb.md), [`vcpu`](vm.md#vcpu-field) field, liveness-requirement bits of vendor structures carried as measured byte sections |
| Tenant identity    | Yes when image-bound; otherwise external | [`sev.id`](sev.md#id-field); CCA RPV and TDX MR\* are runtime-supplied to the VMM, not in PMI     |
| Host identity      | No (PMI acknowledges the channel)        | SEV `HOST_DATA` and equivalents, VMM-supplied                                                     |
| Instance accidents | Yes for resource allocation              | [`dtbo`](vm.md#dtbo-overlay) fill action; everything else (aux granules, EPTP) is VMM-internal    |

Per-target enumerations of every parameter against this table are in
[`sev`](sev.md#parameters), [`cca`](cca.md#parameters), and
[`tdx`](tdx.md#parameters). [`vm`](vm.md) defines the inherited
mechanisms; categories attach where the CC targets pick them up.

## Leftover values

After every PMI- and vendor-defined parameter is walked through the
decision procedure, some values do not fit any of the four identity
categories or instance accidents. The clearest examples are bits
within vendor "policy" or "attributes" fields whose value is the
deployer's operational choice (debug enable, migratability, SMT
allowed, single-socket, ciphertext hiding) — they are host-supplied,
they reach the attestation report through the vendor channel, and
verifier policy checks them, but they bind to neither image nor
tenant nor host nor any platform shape.

These values are real and consequential to a verifier's policy, so
we cannot dismiss them. The leftover set in any one target may
reflect either:

- A **vendor-specific evolutionary quirk** — the vendor design
  bundled an operational choice into a measured or
  attestation-visible field for historical reasons, and PMI's
  conceptual model is still the right way to think about CC. In that
  case the leftover is a documentation finding about that vendor.
- A **gap in PMI's model** — the leftover names a recurring class of
  data across multiple vendors that PMI's five categories do not
  cover, and the model needs a new category (or a refinement of an
  existing one) to accommodate it.

The per-target chapters surface every parameter that does not fit as
**leftover** so the full set is visible in one place; once the
per-target leftover sets are enumerated we can study them together
and decide which leftovers are quirks and which are gaps.

The per-target chapters are the source of truth for which specific
parameters are leftover today.
