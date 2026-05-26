//! `cca` target: Arm CCA confidential virtual machines.

use serde::{Deserialize, Serialize};

use crate::{Target, Version};

pub use crate::kind::{FillKind, LoadKind};

/// `cca` target spec, carried in the `.pmi.cca` PE section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Schema version; MUST be `1`.
    pub version: Version<1>,

    /// Ordered launch recipe.
    pub actions: Vec<Action>,

    /// BSP REC parameters applied via `RMI_REC_CREATE`. CCA is aarch64 only.
    #[serde(rename = "cca:vcpu")]
    pub vcpu: crate::vm::vcpu::aarch64::CpuState,
}

impl Target for Spec {
    const NAME: &'static str = "cca";
    const SECTION: &'static str = ".pmi.cca";
}

/// One entry in the `cca` target's `actions` array.
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
    /// PE section name to load.
    pub section: String,

    /// Load kind; defaults to [`LoadKind::Default`].
    #[serde(default, skip_serializing_if = "LoadKind::is_default")]
    pub kind: LoadKind,
}

/// `fill` action: populate a reserved GPA range with kind-specific content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    /// PE section name to fill (must be a Zero section).
    pub section: String,

    /// Fill kind, selecting how the section is populated.
    pub kind: FillKind,
}
