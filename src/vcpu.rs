use serde::{Deserialize, Serialize};

/// Architecture-discriminated vCPU register map.
///
/// The spec selects the variant by the PE header's `FileHeader.Machine`
/// field; the caller knows the arch before decoding. The two variants have
/// disjoint field sets, so a non-empty map disambiguates structurally.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Vcpu {
    X64(VcpuX64),
    Aarch64(VcpuAarch64),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VcpuX64 {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rip: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rsp: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rflags: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")] pub rax: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rbx: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rcx: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rdx: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rsi: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rdi: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub rbp: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")] pub r8: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub r9: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub r10: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub r11: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub r12: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub r13: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub r14: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub r15: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")] pub cr0: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub cr3: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub cr4: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub efer: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")] pub cs: Option<SegReg>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub ds: Option<SegReg>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub es: Option<SegReg>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub fs: Option<SegReg>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub gs: Option<SegReg>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub ss: Option<SegReg>,

    #[serde(default, skip_serializing_if = "Option::is_none")] pub gdtr: Option<Dtr>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub idtr: Option<Dtr>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegReg {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub selector: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub attributes: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub base: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dtr {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub limit: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub base: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VcpuAarch64 {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x0: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x1: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x2: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x3: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x4: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x5: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x6: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x7: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x8: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x9: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x10: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x11: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x12: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x13: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x14: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x15: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x16: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x17: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x18: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x19: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x20: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x21: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x22: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x23: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x24: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x25: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x26: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x27: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x28: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x29: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub x30: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")] pub sp_el1: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub pc: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub pstate: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")] pub sctlr_el1: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub tcr_el1: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub ttbr0_el1: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub ttbr1_el1: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub mair_el1: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub vbar_el1: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub cpacr_el1: Option<u64>,
}
