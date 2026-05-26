use serde::{Deserialize, Serialize};

use super::is_zero;

// `&u64` is required by serde's `skip_serializing_if`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn rflags_is_default(v: &u64) -> bool {
    *v == 0x2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CpuState {
    #[serde(skip_serializing_if = "is_zero")]
    pub rip: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub rsp: u64,
    #[serde(skip_serializing_if = "rflags_is_default")]
    pub rflags: u64,

    #[serde(skip_serializing_if = "is_zero")]
    pub rax: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub rbx: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub rcx: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub rdx: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub rsi: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub rdi: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub rbp: u64,

    #[serde(skip_serializing_if = "is_zero")]
    pub r8: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub r9: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub r10: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub r11: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub r12: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub r13: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub r14: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub r15: u64,

    #[serde(skip_serializing_if = "is_zero")]
    pub cr0: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub cr3: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub cr4: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub efer: u64,

    #[serde(skip_serializing_if = "is_zero")]
    pub cs: SegReg,
    #[serde(skip_serializing_if = "is_zero")]
    pub ds: SegReg,
    #[serde(skip_serializing_if = "is_zero")]
    pub es: SegReg,
    #[serde(skip_serializing_if = "is_zero")]
    pub fs: SegReg,
    #[serde(skip_serializing_if = "is_zero")]
    pub gs: SegReg,
    #[serde(skip_serializing_if = "is_zero")]
    pub ss: SegReg,

    #[serde(skip_serializing_if = "is_zero")]
    pub gdtr: Dtr,
    #[serde(skip_serializing_if = "is_zero")]
    pub idtr: Dtr,
}

impl Default for CpuState {
    /// All registers zero except `rflags`, which defaults to `0x2`
    /// (the architectural reserved bit, which must be set).
    fn default() -> Self {
        Self {
            rip: 0,
            rsp: 0,
            rflags: 0x2,
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            cr0: 0,
            cr3: 0,
            cr4: 0,
            efer: 0,
            cs: SegReg::default(),
            ds: SegReg::default(),
            es: SegReg::default(),
            fs: SegReg::default(),
            gs: SegReg::default(),
            ss: SegReg::default(),
            gdtr: Dtr::default(),
            idtr: Dtr::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SegReg {
    #[serde(skip_serializing_if = "is_zero")]
    pub selector: u16,
    #[serde(skip_serializing_if = "is_zero")]
    pub attributes: u16,
    #[serde(skip_serializing_if = "is_zero")]
    pub limit: u32,
    #[serde(skip_serializing_if = "is_zero")]
    pub base: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Dtr {
    #[serde(skip_serializing_if = "is_zero")]
    pub limit: u16,
    #[serde(skip_serializing_if = "is_zero")]
    pub base: u64,
}
