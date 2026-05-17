# Why PMI?

## Boot Modes

The Linux ecosystem boots machines in a variety of ways. Unlike Windows, which
assumes a single UEFI boot path, Linux deployers routinely use direct kernel
loading, firmware passthrough, service modules, and combinations of all three. A
machine boots by combining three components, each of which may be absent,
provided by the host, provided by the tenant (bundled in the image), or loaded
from disk:

| Mode        | Service |    Firmware     |  Kernel   | BM  | VM  | CVM |
| :---------- | :-----: | :-------------: | :-------: | --- | --- | --- |
| Extracted   |         |                 | extracted |     | ✓   | ✓   |
| Stubbed     |         | vm: yes, bm: no |  stubbed  | ✓   | ✓   | ✓   |
| Traditional |         |       yes       |  on disk  |     | ✓   | ✓   |
| Serviced    |   yes   |       yes       |  on disk  |     |     | ✓   |

1. **Extracted** This mode applies only to virtual machines. In this mode, the
   VMM takes the role of guest firmware. It extracts the kernel from the PE,
   loads it into guest memory according to the Linux boot protocol, and starts
   the guest at the kernel entry point. No firmware is involved. An example of
   this mode is `qemu -kernel image.efi`.

2. **Stubbed** This mode applies to both bare metal and virtual machines. UEFI
   is required for this model. UEFI executes the PE. In the case of a UKI, the
   PE contains an EFI stub and a kernel. The EFI stub loads the kernel into
   memory and starts it. This works on bare metal via PXE, and via UEFI HTTP
   Boot. In a VM environment, this method can be used but requires a guest UEFI
   implementation (e.g., OVMF). An example of this method is
   `qemu -bios OVMF.fd -kernel image.efi` which boots the guest with just OVMF
   and then passes the UKI to the guest over the `fw_cfg` interface.

3. **Traditional** This mode applies to both bare metal and virtual machines.
   UEFI is required for this model. UEFI executes the PE, but the PE contains no
   kernel — only firmware (if any) and the manifest. The firmware loads the
   kernel from disk and starts it. This works on bare metal via PXE, and via
   UEFI HTTP Boot. In a VM environment, this method can be used but requires a
   guest UEFI implementation (e.g., OVMF). An example of this method is
   `qemu -bios OVMF.fd -kernel image.efi` which boots the guest with just OVMF
   and then passes the manifest to the guest over the `fw_cfg` interface. The
   manifest tells OVMF where to find the kernel on disk and how to boot it.

4. **Serviced** This mode applies only to confidential virtual machines. The
   tenant provides a service module and firmware bundled in the image. The
   host's VMM launches the service module as the privileged layer, which
   initializes the confidential environment and then launches the tenant's
   firmware. The tenant's firmware boots the kernel, usually from disk, and
   measures it using a vTPM provided by the service layer. An example of this
   mode is COCONUT-SVSM + OVMF, where the SVSM service module initializes SEV
   3.0 and then launches OVMF, which boots the kernel from disk.

## Format Comparison

No existing format covers all of these modes:

- **PE** is the universal UEFI boot image, but has no virtualization or
  confidential computing semantics on its own.

- **UKI** (PE + kernel + EFI stub) adds VMM direct boot support via the Linux
  boot protocol, but cannot carry firmware, service modules, or CC metadata.

- **IGVM** provides full CC semantics and can carry firmware and service
  modules, but is not a PE — it cannot boot on bare metal, via PXE, or via UEFI
  HTTP boot.

| Mode        | PE  | UKI | IGVM | PMI |
| :---------- | :-: | :-: | :--: | :-: |
| Direct      |     |  ✓  |  ✓   |  ✓  |
| Bundled     |  ✓  |  ✓  |      |  ✓  |
| Traditional |     |     |  ✓   |  ✓  |
| Serviced    |     |     |  ✓   |  ✓  |

Like UKI, PMI is a strict superset of PE. It inherits bare metal and direct boot
from PE, adds every capability IGVM provides, and remains a valid PE throughout.
A single build pipeline produces one artifact for all deployment targets.

## Why Not IGVM?

PMI is inspired by IGVM. IGVM is a well-designed format for its original
purpose: describing confidential guest paravisor images for VMMs. However, PMI
exists because that purpose is too narrow.

1. **IGVM is not bootable on UEFI.** IGVM can carry a kernel, but the resulting
   image cannot boot on UEFI. UEFI firmware cannot load it. PXE cannot chainload
   it. HTTP Boot cannot fetch and execute it. Any deployment that touches bare
   metal or UEFI needs a separate image and a separate build pipeline. PMI is a
   PE — the same artifact boots on bare metal, in a VM, and in a confidential
   VM.

2. **IGVM encodes page-level load commands.** An IGVM file breaks contiguous
   memory regions into 4K page directives that are not aligned on disk. This
   makes `mmap()` impossible and huge pages require a copy. In testing, this
   added ~100ms of boot latency for a bundled linux kernel. PMI expresses
   regions, not pages. Sections are aligned on disk (4K or 2M), so the VMM can
   `mmap()` the file and pass sections directly to platform APIs using huge
   pages with zero copy.

3. **IGVM defines proprietary VMM interfaces.** Specifically, it creates an
   IGVM-specific memory map instead of reusing standard formats (Devicetree,
   EFI memory map) already understood by VMMs and guests.

4. **IGVM couples data and policy.** IGVM directives mix guest memory contents,
   page types, measurement boundaries, and platform policy into a single ordered
   stream. Changing the SEV policy means re-serializing the directive stream.
   Adding a new section means inserting directives at the correct position among
   platform-specific pages. There is no way to supply policy externally — it is
   baked into the directive stream at build time.

5. **IGVM requires format-aware tooling.** You cannot inspect or modify an IGVM
   file with standard PE tools. objcopy, readelf, sbsign, and systemd-ukify do
   not apply. PMI images are PE files — the existing toolchain continues to
   work.

PMI addresses these limitations while retaining everything IGVM gets right:
explicit VMM instructions, CC platform support, and deterministic measurement.
