// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]

pub mod aarch64;
pub mod x86_64;

use core::fmt;

fn is_zero<T: Default + PartialEq>(v: &T) -> bool {
    v == &T::default()
}

/// A boot-vCPU register value that violates an architectural invariant the
/// spec pins (`spec/vm.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidRegister {
    /// `rflags` bit 1 (reserved) was clear; the architecture requires it set.
    RflagsReservedBitClear,

    /// A segment-attribute word set reserved bits 12–15, which must be zero.
    SegmentReservedBits,

    /// `pstate` does not select EL1 (`M[3:0]` is neither `EL1t` nor `EL1h`).
    PstateNotEl1,
}

impl fmt::Display for InvalidRegister {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::RflagsReservedBitClear => "rflags bit 1 (reserved) must be set",
            Self::SegmentReservedBits => {
                "segment attribute bits 12-15 are reserved and must be zero"
            }
            Self::PstateNotEl1 => "pstate must select EL1 (M[3:0] = 0b0100 or 0b0101)",
        })
    }
}

impl std::error::Error for InvalidRegister {}
