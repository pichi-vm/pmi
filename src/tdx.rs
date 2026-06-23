// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! `tdx` target: Intel TDX confidential virtual machines.

use serde::{Deserialize, Serialize};

use crate::{Target, Version};

pub use crate::cpu::Profile;
pub use crate::kind::{FillKind, LoadKind};

/// `tdx` target spec, carried in the `.pmi.tdx` PE section.
///
/// The `tdx` target defines no `vm:vcpu`: TDX fixes the initial register state
/// in the module, and the measured consumer derives platform facts from
/// `TDG.VP.INFO` rather than from registers (see `spec/tdx.md`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Schema version; MUST be `1`.
    pub version: Version<1>,

    /// Ordered launch recipe.
    pub actions: Vec<Action>,

    /// vCPU ISA baseline (`cpu:profile` target attribute).
    #[serde(rename = "cpu:profile")]
    pub cpu_profile: Profile,

    /// Optional `merged:dtb` target attribute: PE section name holding the
    /// base DTB when this image uses the `merged` extension. Required when
    /// `actions` contains a `merged:dtbo` fill; absent otherwise.
    #[serde(
        rename = "merged:dtb",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub merged_dtb: Option<String>,
}

impl Target for Spec {
    const NAME: &'static str = "tdx";
    const SECTION: &'static str = ".pmi.tdx";
}

/// One entry in the `tdx` target's `actions` array.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    /// `load` action.
    Load(Load),
    /// `fill` action.
    Fill(Fill),
}

/// `load` action: place a PE section's bytes into guest memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Load {
    /// Absolute guest-physical address at which the section's bytes are placed.
    pub gpa: u64,

    /// PE section supplying the bytes.
    pub section: String,

    /// Load kind; defaults to [`LoadKind::Default`].
    #[serde(default, skip_serializing_if = "LoadKind::is_default")]
    pub kind: LoadKind,
}

/// `fill` action: populate a reserved GPA range with kind-specific content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    /// Absolute guest-physical address of the filled range.
    pub gpa: u64,

    /// PE section name to fill (must be a Zero section).
    pub section: String,

    /// Fill kind, selecting how the section is populated.
    pub kind: FillKind,
}
