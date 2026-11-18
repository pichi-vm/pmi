// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! `vm` target: non-confidential virtual machines.

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{Target, Version};

pub use crate::kind::{FillKind, LoadKind};

/// `vm` target spec, carried in the `.pmi.vm` PE section.
///
/// `P` is the `cpu:profile` type; it MUST match `PE.FileHeader.Machine`. Use
/// [`crate::x86_64::Profile`] for `0x8664` and [`crate::aarch64::Profile`] for
/// `0xAA64`. The caller selects it from the PE header before decoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    deny_unknown_fields,
    bound(serialize = "P: Serialize + Default + PartialEq")
)]
pub struct Spec<P> {
    /// Schema version; MUST be `1`.
    pub version: Version<1>,

    /// Ordered launch recipe.
    pub actions: Vec<Action>,

    /// Effective vCPU ISA baseline; defaults to the architecture baseline.
    #[serde(
        rename = "cpu:profile",
        default,
        skip_serializing_if = "crate::IsDefault::is_default"
    )]
    pub cpu_profile: P,

    /// Optional `dt:dtb` target attribute: PE section holding the bundled
    /// base DTB. Present means a base DTB is bundled in the image, consumed by
    /// a `load` or as the default source of a `dt:dtb` fill; absent means the
    /// base is distributed separately (detached mode).
    #[serde(rename = "dt:dtb", default, skip_serializing_if = "Option::is_none")]
    pub dt_dtb: Option<String>,
}

impl<P: DeserializeOwned + Serialize + Default + PartialEq> Target for Spec<P> {
    const NAME: &'static str = "vm";
    const SECTION: &'static str = ".pmi.vm";
}

/// One entry in the `vm` target's `actions` array.
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
