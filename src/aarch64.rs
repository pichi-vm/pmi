// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! aarch64: the architecture-specific parts of PMI (`spec/aarch64.md`).

use serde::{Deserialize, Serialize};

/// aarch64 `cpu:profile` value. Serializes as its name in the spec.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Profile {
    /// `armv8.0-a`.
    #[serde(rename = "armv8.0-a")]
    #[default]
    V8_0,
    /// `armv8.1-a`.
    #[serde(rename = "armv8.1-a")]
    V8_1,
    /// `armv8.2-a`.
    #[serde(rename = "armv8.2-a")]
    V8_2,
    /// `armv8.3-a`.
    #[serde(rename = "armv8.3-a")]
    V8_3,
    /// `armv8.4-a`.
    #[serde(rename = "armv8.4-a")]
    V8_4,
    /// `armv8.5-a`.
    #[serde(rename = "armv8.5-a")]
    V8_5,
    /// `armv8.6-a`.
    #[serde(rename = "armv8.6-a")]
    V8_6,
    /// `armv8.7-a`.
    #[serde(rename = "armv8.7-a")]
    V8_7,
    /// `armv8.8-a`.
    #[serde(rename = "armv8.8-a")]
    V8_8,
    /// `armv8.9-a`.
    #[serde(rename = "armv8.9-a")]
    V8_9,
    /// `armv9.0-a`.
    #[serde(rename = "armv9.0-a")]
    V9_0,
    /// `armv9.1-a`.
    #[serde(rename = "armv9.1-a")]
    V9_1,
    /// `armv9.2-a`.
    #[serde(rename = "armv9.2-a")]
    V9_2,
    /// `armv9.3-a`.
    #[serde(rename = "armv9.3-a")]
    V9_3,
    /// `armv9.4-a`.
    #[serde(rename = "armv9.4-a")]
    V9_4,
    /// `armv9.5-a`.
    #[serde(rename = "armv9.5-a")]
    V9_5,
    /// `armv9.6-a`.
    #[serde(rename = "armv9.6-a")]
    V9_6,
    /// `armv9.7-a`.
    #[serde(rename = "armv9.7-a")]
    V9_7,
    /// `armv9.8-a`.
    #[serde(rename = "armv9.8-a")]
    V9_8,
}
