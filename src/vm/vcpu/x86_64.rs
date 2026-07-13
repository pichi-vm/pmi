// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{de, Deserialize, Deserializer, Serialize};

use super::{is_zero, InvalidRegister};

/// x86-64 `RFLAGS`, with bit 1 (architecturally reserved-as-one) guaranteed
/// set. Serializes as the underlying `u64`; construction and deserialization
/// reject any value that clears bit 1 (`spec/vm.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RFlags(u64);

impl RFlags {
    /// Construct from raw bits.
    ///
    /// # Errors
    /// [`InvalidRegister::RflagsReservedBitClear`] if bit 1 is not set.
    pub fn new(bits: u64) -> Result<Self, InvalidRegister> {
        if bits & 0x2 == 0 {
            return Err(InvalidRegister::RflagsReservedBitClear);
        }
        Ok(Self(bits))
    }

    /// The raw `RFLAGS` value.
    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}

impl Default for RFlags {
    /// `0x2` — the reserved-as-one bit set, all others clear.
    fn default() -> Self {
        Self(0x2)
    }
}

impl TryFrom<u64> for RFlags {
    type Error = InvalidRegister;

    fn try_from(bits: u64) -> Result<Self, Self::Error> {
        Self::new(bits)
    }
}

impl<'de> Deserialize<'de> for RFlags {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(u64::deserialize(d)?).map_err(de::Error::custom)
    }
}

/// A segment-register attribute word, with reserved bits 12–15 guaranteed zero
/// (`spec/vm.md`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SegAttributes(u16);

impl SegAttributes {
    /// Construct from raw bits.
    ///
    /// # Errors
    /// [`InvalidRegister::SegmentReservedBits`] if any of bits 12–15 are set.
    pub fn new(bits: u16) -> Result<Self, InvalidRegister> {
        if bits & 0xF000 != 0 {
            return Err(InvalidRegister::SegmentReservedBits);
        }
        Ok(Self(bits))
    }

    /// The raw attribute word.
    #[must_use]
    pub fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for SegAttributes {
    type Error = InvalidRegister;

    fn try_from(bits: u16) -> Result<Self, Self::Error> {
        Self::new(bits)
    }
}

impl<'de> Deserialize<'de> for SegAttributes {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(u16::deserialize(d)?).map_err(de::Error::custom)
    }
}

// `&RFlags` is required by serde's `skip_serializing_if`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn rflags_is_default(v: &RFlags) -> bool {
    *v == RFlags::default()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CpuState {
    #[serde(skip_serializing_if = "is_zero")]
    pub rip: u64,
    #[serde(skip_serializing_if = "is_zero")]
    pub rsp: u64,
    #[serde(skip_serializing_if = "rflags_is_default")]
    pub rflags: RFlags,

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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SegReg {
    #[serde(skip_serializing_if = "is_zero")]
    pub selector: u16,
    #[serde(skip_serializing_if = "is_zero")]
    pub attributes: SegAttributes,
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

#[cfg(test)]
mod tests {
    use super::{CpuState, RFlags, SegAttributes};

    #[test]
    fn default_sets_only_the_rflags_reserved_bit() {
        let s = CpuState::default();
        assert_eq!(s.rflags.get(), 0x2, "rflags bit 1 must be set by default");
        assert_eq!(s.rip, 0);
        assert_eq!(s.rsp, 0);
        assert_eq!(s.cr0, 0);
    }

    #[test]
    fn rflags_rejects_a_clear_reserved_bit() {
        assert!(RFlags::new(0).is_err());
        assert!(RFlags::new(0x1).is_err(), "bit 1 clear must be rejected");
        assert_eq!(RFlags::new(0x202).unwrap().get(), 0x202);
        assert_eq!(RFlags::default().get(), 0x2);
    }

    #[test]
    fn segment_attributes_reject_reserved_bits() {
        assert!(SegAttributes::new(0x1000).is_err());
        assert_eq!(SegAttributes::new(0x0F93).unwrap().get(), 0x0F93);
    }
}
