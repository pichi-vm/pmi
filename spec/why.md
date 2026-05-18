# Why PMI?

PMI exists to solve two problems with the way virtual machine images are
defined and consumed today: the platform-definition inversion and the
single-artifact problem.

## The platform-definition inversion

The historical pattern for booting a machine is: firmware defines the
platform layout, guest software adapts to it. The firmware enumerates
devices, decides where memory lives, exposes a CPU configuration, and hands
that picture to the kernel through a well-known interface (a Devicetree, an
ACPI table set, an `e820` map, etc.). The guest reads the interface and
configures itself accordingly.

On bare metal this asymmetry made sense. The firmware ran first, had direct
knowledge of the underlying hardware, and was structurally positioned to
discover the platform. The guest had limited capability to express what it
required in early boot, and bare-metal hardware was largely fixed regardless
of what the guest might have wanted.

Virtual machines flip the capability asymmetry.

A hypervisor has near-arbitrary flexibility to compose any platform the
guest will see — any set of virtual devices, any memory map, any CPUID
exposure, any interrupt controller version. The guest, meanwhile, has
essentially the same early-boot constraints it had on bare metal: it cannot
re-verify the platform after the fact, cannot defend itself against subtle
configuration discrepancies, and is at the mercy of whatever the hypervisor
chose to present. The party with the most flexibility is the one whose
choices the other party cannot effectively check.

Confidential Computing extends this into a security boundary. The
hypervisor is no longer trusted, but the guest still consumes the platform
definition the hypervisor produces. A maliciously-crafted DSDT, an
unexpected MMIO region, a missing or substituted device — the guest has no
practical way to defend against these in early boot. Concrete
demonstrations of this attack surface exist:

- [AMD-SB-3012](https://www.amd.com/en/resources/product-security/bulletin/amd-sb-3012.html)
  — ACPI/AML injection in SEV guests via QEMU.
- [BadAML](https://dl.acm.org/doi/10.1145/3719027.3765123) (ACM CCS 2025,
  Distinguished Paper) — universal AML injection across SEV and TDX guests.

PMI inverts the model. The image declares the platform layout it requires
— devices, memory map, MMIO regions, PCIe topology, interrupt controllers
— via a base [DTB](dtb.md) it carries. The VMM is obligated to provide
exactly what the image declares, or refuse to launch. The party with the
flexibility loses the ability to use it unilaterally; the guest no longer
needs to verify after the fact because the contract is established before
launch and bound to the attestation.

## The single-artifact problem

Linux boots through more shapes than bare metal does. On bare metal the
pipeline is roughly _firmware → kernel_. In a virtual machine the pipeline
grows to _firmware (UEFI) → hypervisor → (optional service module) →
(optional firmware) → kernel_, and the kernel itself may be embedded in the
image, loaded from disk, or extracted by the hypervisor.

![Boot pipelines: bare metal versus modern VM](images/boot-modes.excalidraw.svg)

A **service module** is a Confidential Computing construct: a privileged
component the host's VMM launches before the guest firmware, which
initializes the confidential environment, provides a vTPM, and then drops
the guest firmware down to a lower privilege level. The canonical example
is [COCONUT-SVSM](https://github.com/coconut-svsm/svsm) on AMD SEV-SNP,
which runs at VMPL0 and exposes services to OVMF at VMPL1. Service modules
are absent from bare metal and non-CC VM boot — they exist solely to bridge
the trust transition that CC targets introduce.

Unlike Windows, which assumes a single UEFI boot path, Linux deployers
routinely use direct kernel loading, firmware passthrough, service modules,
and combinations of all three. A machine boots by combining three
components, each of which may be absent, provided by the host, provided by
the tenant (bundled in the image), or loaded from disk:

| Mode        | Service |    Firmware     |  Kernel   | BM  | VM  | CVM |
| :---------- | :-----: | :-------------: | :-------: | --- | --- | --- |
| Extracted   |         |                 | extracted |     | ✓   | ✓   |
| Stubbed     |         | vm: yes, bm: no |  stubbed  | ✓   | ✓   | ✓   |
| Traditional |         |       yes       |  on disk  |     | ✓   | ✓   |
| Serviced    |   yes   |       yes       |  on disk  |     |     | ✓   |

1. **Extracted** — VM only. The VMM takes the role of guest firmware. It
   extracts the kernel from the PE, loads it into guest memory according to
   the Linux boot protocol, and starts the guest at the kernel entry point.
   No firmware is involved. Example: `qemu -kernel image.efi`.

2. **Stubbed** — bare metal or VM. UEFI executes the PE. The PE contains an
   EFI stub and a kernel (UKI shape); the stub loads the kernel into memory
   and starts it. Works on bare metal via PXE and UEFI HTTP Boot. In a VM,
   requires a guest UEFI implementation (e.g., OVMF). Example:
   `qemu -bios OVMF.fd -kernel image.efi` boots OVMF and passes the UKI
   over `fw_cfg`.

3. **Traditional** — bare metal or VM. UEFI executes the PE, but the PE
   carries no kernel — only firmware and boot configuration. The firmware
   loads the kernel from disk. Works on bare metal via PXE and UEFI HTTP
   Boot. In a VM, requires a guest UEFI implementation.

4. **Serviced** — CVM only. The tenant provides a service module and
   firmware bundled in the image. The host's VMM launches the service
   module as the privileged layer, which initializes the confidential
   environment and then launches the tenant's firmware. The tenant's
   firmware boots the kernel, usually from disk, and measures it using a
   vTPM provided by the service layer. Example: COCONUT-SVSM + OVMF.

Historically each mode required its own build pipeline and image format —
PE for UEFI boot, UKI for VMs that direct-boot, IGVM for paravisor-style
confidential boot. Producing an image that worked across modes meant
producing several images.

PMI is a strict superset of PE. The same PE binary boots on bare metal,
in a non-CC VM, and in a confidential VM on multiple CC targets. Per-target
launch recipes are carried in their own non-loaded PE sections (by
convention `.pmi.<target>`) — UEFI ignores them and boots the PE the way it
already knows how. A VMM that understands PMI reads the section for the
target it picks and executes that recipe.

| Mode        | PE  | UKI | IGVM | PMI |
| :---------- | :-: | :-: | :--: | :-: |
| Extracted   |     |  ✓  |  ✓   |  ✓  |
| Stubbed     |  ✓  |  ✓  |      |  ✓  |
| Traditional |     |     |  ✓   |  ✓  |
| Serviced    |     |     |  ✓   |  ✓  |

## Relation to IGVM

PMI is inspired by IGVM, which addresses the same family of problems for a
narrower scope (loading paravisor-style confidential guest images for
specific VMMs). PMI's broader scope drove several design choices that
differ from IGVM's:

- PMI is a PE, so the same artifact boots through UEFI, PXE, and HTTP Boot
  paths in addition to VM and CVM paths.
- PMI describes regions rather than per-page load commands, allowing
  zero-copy `mmap()` loading and trivial use of huge pages.
- PMI reuses standard interfaces (Devicetree for platform topology) rather
  than defining VMM-specific structures for the same information.
- PMI separates the image's launch recipe from the deployer's policy inputs
  (e.g., SEV `host_data`), so the same image can be deployed with different
  external configuration without rebuilding.
- PMI images are inspectable and modifiable with standard PE tooling
  (`objcopy`, `sbsign`, `systemd-ukify`).

For the narrower problem IGVM was designed for, IGVM remains the
better-fitting tool. PMI exists to cover the broader set of deployment
shapes Linux ecosystems actually use.
