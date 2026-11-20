// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! aarch64: the architecture-specific parts of PMI (`spec/aarch64.md`).

use serde::{Deserialize, Serialize};
use strum::{EnumString, IntoStaticStr};

/// aarch64 `cpu:profile` value. Serializes as its name in the spec.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, IntoStaticStr,
)]
#[serde(try_from = "String", into = "&'static str")]
pub enum Profile {
    /// `armv8.0-a`.
    #[strum(serialize = "armv8.0-a")]
    #[default]
    V8_0,
    /// `armv8.1-a`.
    #[strum(serialize = "armv8.1-a")]
    V8_1,
    /// `armv8.2-a`.
    #[strum(serialize = "armv8.2-a")]
    V8_2,
    /// `armv8.3-a`.
    #[strum(serialize = "armv8.3-a")]
    V8_3,
    /// `armv8.4-a`.
    #[strum(serialize = "armv8.4-a")]
    V8_4,
    /// `armv8.5-a`.
    #[strum(serialize = "armv8.5-a")]
    V8_5,
    /// `armv8.6-a`.
    #[strum(serialize = "armv8.6-a")]
    V8_6,
    /// `armv8.7-a`.
    #[strum(serialize = "armv8.7-a")]
    V8_7,
    /// `armv8.8-a`.
    #[strum(serialize = "armv8.8-a")]
    V8_8,
    /// `armv8.9-a`.
    #[strum(serialize = "armv8.9-a")]
    V8_9,
    /// `armv9.0-a`.
    #[strum(serialize = "armv9.0-a")]
    V9_0,
    /// `armv9.1-a`.
    #[strum(serialize = "armv9.1-a")]
    V9_1,
    /// `armv9.2-a`.
    #[strum(serialize = "armv9.2-a")]
    V9_2,
    /// `armv9.3-a`.
    #[strum(serialize = "armv9.3-a")]
    V9_3,
    /// `armv9.4-a`.
    #[strum(serialize = "armv9.4-a")]
    V9_4,
    /// `armv9.5-a`.
    #[strum(serialize = "armv9.5-a")]
    V9_5,
    /// `armv9.6-a`.
    #[strum(serialize = "armv9.6-a")]
    V9_6,
    /// `armv9.7-a`.
    #[strum(serialize = "armv9.7-a")]
    V9_7,
    /// `armv9.8-a`.
    #[strum(serialize = "armv9.8-a")]
    V9_8,
}

impl TryFrom<String> for Profile {
    type Error = strum::ParseError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        name.parse()
    }
}
