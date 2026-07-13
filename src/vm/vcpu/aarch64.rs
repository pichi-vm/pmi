// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{de, Deserialize, Deserializer, Serialize};

use super::{is_zero, InvalidRegister};

/// `AArch64` `PSTATE` (in SPSR form), constrained to select EL1.
///
/// EL1 means `M[3:0]` is `0b0100` (`EL1t`) or `0b0101` (`EL1h`), which also
/// fixes `M[4] = 0` (`AArch64`). There is no `Default`: the spec makes `pstate`
/// required because no single value is universally correct (`spec/vm.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PState(u64);

impl PState {
    /// Construct from raw bits.
    ///
    /// # Errors
    /// [`InvalidRegister::PstateNotEl1`] if `M[3:0]` is neither `EL1t` nor `EL1h`.
    pub fn new(bits: u64) -> Result<Self, InvalidRegister> {
        match bits & 0x1F {
            0x4 | 0x5 => Ok(Self(bits)),
            _ => Err(InvalidRegister::PstateNotEl1),
        }
    }

    /// The raw `PSTATE` value.
    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for PState {
    type Error = InvalidRegister;

    fn try_from(bits: u64) -> Result<Self, Self::Error> {
        Self::new(bits)
    }
}

impl<'de> Deserialize<'de> for PState {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(u64::deserialize(d)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuState {
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x0: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x1: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x2: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x3: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x4: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x5: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x6: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x7: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x8: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x9: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x10: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x11: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x12: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x13: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x14: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x15: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x16: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x17: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x18: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x19: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x20: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x21: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x22: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x23: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x24: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x25: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x26: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x27: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x28: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x29: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x30: u64,

    #[serde(default, skip_serializing_if = "is_zero")]
    pub sp_el1: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pc: u64,

    /// `PSTATE`; required, and MUST select EL1 (`spec/vm.md`).
    pub pstate: PState,

    #[serde(default, skip_serializing_if = "is_zero")]
    pub sctlr_el1: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tcr_el1: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub ttbr0_el1: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub ttbr1_el1: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub mair_el1: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub vbar_el1: u64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cpacr_el1: u64,
}

#[cfg(test)]
mod tests {
    use super::PState;

    #[test]
    fn pstate_accepts_only_el1() {
        assert_eq!(PState::new(0x5).unwrap().get(), 0x5); // EL1h
        assert_eq!(PState::new(0x4).unwrap().get(), 0x4); // EL1t
        assert!(PState::new(0x0).is_err()); // EL0t
        assert!(PState::new(0x9).is_err()); // EL2h
                                            // DAIF masks (bits 6-9) do not affect EL selection.
        assert!(PState::new(0x3C5).is_ok());
    }
}
