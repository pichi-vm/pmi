use serde::{Deserialize, Serialize};

use super::is_zero;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CpuState {
    #[serde(skip_serializing_if = "is_zero")]
    pub x0: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x1: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x2: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x3: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x4: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x5: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x6: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x7: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x8: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x9: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x10: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x11: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x12: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x13: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x14: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x15: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x16: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x17: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x18: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x19: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x20: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x21: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x22: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x23: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x24: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x25: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x26: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x27: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x28: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x29: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub x30: u64,

    #[serde(skip_serializing_if = "is_zero")]
    pub sp_el1: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub pc: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub pstate: u64,

    #[serde(skip_serializing_if = "is_zero")]
    pub sctlr_el1: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub tcr_el1: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub ttbr0_el1: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub ttbr1_el1: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub mair_el1: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub vbar_el1: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub cpacr_el1: u64,
}
