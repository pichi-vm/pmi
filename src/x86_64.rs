// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! x86-64: the architecture-specific parts of PMI (`spec/x86-64.md`).

use serde::{Deserialize, Serialize};
use strum::{EnumString, IntoStaticStr};

/// x86-64 `cpu:profile` value. Serializes as its name in the spec.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, IntoStaticStr,
)]
#[serde(try_from = "String", into = "&'static str")]
pub enum Profile {
    /// `x86-64-v1`, the psABI baseline.
    #[strum(serialize = "x86-64-v1")]
    #[default]
    V1,
    /// `x86-64-v2`.
    #[strum(serialize = "x86-64-v2")]
    V2,
    /// `x86-64-v3`.
    #[strum(serialize = "x86-64-v3")]
    V3,
    /// `x86-64-v4`.
    #[strum(serialize = "x86-64-v4")]
    V4,
}

impl TryFrom<String> for Profile {
    type Error = strum::ParseError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        name.parse()
    }
}
