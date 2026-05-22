# Categories

PMI distinguishes five categories of data that flow through any
launch. The categories give the spec a uniform vocabulary for talking
about what gets measured, what reaches the attestation report through
side channels, what is image-bound, what is deployer-bound, and what
has no identity meaning at all.

This page defines each category in depth, calls out a sixth class of
input — launch policy — that deliberately is **not** a PMI category,
and gives a decision procedure for classifying any new target-specific
parameter. Each per-target chapter ([`vm`](vm.md), [`sev`](sev.md),
[`cca`](cca.md), [`tdx`](tdx.md)) enumerates its own parameters
against this framework.

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

## Launch policy is not a PMI category

Vendor APIs typically carry a field labeled "policy" or
"attributes" that the deployer passes in at launch
(`SNP_LAUNCH_START`'s POLICY, TDX's `ATTRIBUTES`,
RmiRealmParams's feature flags). Within a single such field, the
individual bits may belong to **different** PMI categories:

- **Liveness-requirement bits** — bits whose value the image
  depends on to run correctly (e.g., the TDX `ATTRIBUTES` bits that
  enable LASS or PKS or PERFMON, the `XFAM` bits that authorize
  SVE/AVX use). These are **platform identity** — they describe the
  shape of the hardware the image expects.

- **Launch policy bits** — bits the deployer chooses for
  operational reasons that don't change what the image needs from
  the platform (e.g., SEV-SNP's `POLICY.DEBUG` /
  `POLICY.MIGRATE_MA` / `POLICY.SMT`, TDX's `ATTRIBUTES.DEBUG` /
  `ATTRIBUTES.MIGRATABLE`). These are **out of PMI scope** as a
  category. They reach the firmware via the vendor's channel and
  appear in the attestation report; verifiers check them against
  deployer policy.

Launch policy bits are not image identity, not platform identity,
not tenant identity, not host identity, and not instance accidents.
They are a sixth class that PMI deliberately does **not** carry.
The reason: PMI's job is to bind what the image declares to what
the launch measures. Launch policy is a deployer choice about
*how* to run a given image; it is host-supplied via vendor
channels and surfaced in the attestation report by vendor
mechanisms. PMI adds nothing by re-routing it.

When a single vendor field mixes liveness requirements and launch
policy in one bit-field (TDX's `ATTRIBUTES` is the running
example), the per-target chapter MUST split the field bit by bit
and assign each bit to its category.

## Decision procedure

For any new target-specific parameter, classify it by answering
these questions in order:

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

6. **Otherwise — is this a deployer operational choice that
   doesn't change the workload's identity but does appear in the
   attestation report?** (Debug enable, migratability,
   SMT-allowed.) → **launch policy**, out of PMI scope.

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
| Launch policy      | Deliberately no                          | Vendor-defined fields passed to firmware by VMM-supplied input; appear in attestation report      |

Per-target enumerations of every parameter against this table are in
[`vm`](vm.md#parameters), [`sev`](sev.md#parameters),
[`cca`](cca.md#parameters), and [`tdx`](tdx.md#parameters).
