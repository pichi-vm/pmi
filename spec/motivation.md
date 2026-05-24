# Motivation

A single workload increasingly needs to run as more than one shape
of artifact. The same kernel + initrd + command line is expected
to launch on bare metal under UEFI, as a direct-boot VM where the
VMM extracts the kernel, under guest firmware (OVMF) that loads
the kernel from a virtual disk, and inside a confidential VM under
SEV-SNP, TDX, or CCA. Each of those shapes has its own image
format conventions today (PE/UKI assumes UEFI, the Linux boot
protocol assumes VMM extraction, IGVM assumes paravisor-style
confidential boot, every cloud has ad-hoc SEV/TDX conventions),
which forces image authors to publish N artifacts where they
wanted to publish one. PMI's response: a single PE binary that
declares one launch recipe per target it supports, with PE-level
selection so the same bytes work everywhere.

Each confidential-computing target ships its own conventions for
the launch recipe — Intel TDX HOBs, IGVM, QEMU/libkrun/cloud
hypervisor variants. None compose. A workload that wants to run
SEV on AWS, TDX on Azure, and CCA on tenant-owned Arm hardware
ends up maintaining three image-build paths, three verifier
integrations, three consumer stubs. PMI's response: one CBOR wire
format and one action mechanism that drives every target's native
firmware ABI, so an image author learns one shape and ships one
image to N targets.

Image formats are not consumed by one tool; producers, VMMs,
in-guest stubs, verifiers, inspectors, and signers all touch them.
Replacing PE would force every layer to be re-implemented and lose
decades of operational maturity (`objcopy`, `sbsign`, `ukify`,
`systemd-stub`, every UEFI loader). PMI's response: extend PE
rather than replace it, so the existing toolchain works on PMI
images unchanged, and the new tooling (CBOR parser, signer,
inspector) has narrow contracts that compose across targets.

The substrate scope is deliberately narrow: PE container + per-
target launch recipes + action mechanism. Platform semantics,
attestation policy, host-conformance checking, and the
measured-vs-unmeasured boundary as a security argument belong to
upper-layer specs (e.g., dillo) that build on top of PMI.
