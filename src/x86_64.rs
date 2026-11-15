// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! x86-64: the architecture-specific parts of PMI (`spec/x86-64.md`).

use serde::{Deserialize, Serialize};

/// x86-64 `cpu:profile` value. Serializes as its name in the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Profile {
    /// `x86-64-v1`, the psABI baseline.
    #[serde(rename = "x86-64-v1")]
    V1,
    /// `x86-64-v2`.
    #[serde(rename = "x86-64-v2")]
    V2,
    /// `x86-64-v3`.
    #[serde(rename = "x86-64-v3")]
    V3,
    /// `x86-64-v4`.
    #[serde(rename = "x86-64-v4")]
    V4,
}
