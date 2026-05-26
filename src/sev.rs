//! `sev` target: AMD SEV-SNP confidential virtual machines.

use serde::{Deserialize, Serialize};

use crate::{Target, Version};

/// `sev` target spec, carried in the `.pmi.sev` PE section.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    /// Schema version; MUST be `1`.
    pub version: Version<1>,

    /// Ordered launch recipe.
    pub actions: Vec<Action>,

    /// Optional signed launch identity. Present on signed launches.
    #[serde(rename = "sev:id", default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

impl Target for Spec {
    const NAME: &'static str = "sev";
    const SECTION: &'static str = ".pmi.sev";
}

/// Signed launch identity: PE-section names for the 96-byte ID block and
/// the 4 KiB ID auth info.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Id {
    /// PE section carrying the 96-byte SEV-SNP ID block.
    pub block: String,

    /// PE section carrying the 4 KiB SEV-SNP ID auth info structure.
    pub auth: String,
}

/// One entry in the `sev` target's `actions` array.
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

/// `load` action kinds accepted on the `sev` target: the core `default`,
/// plus `sev:vmsa` for the BSP VMSA page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadKind {
    /// Core `default` kind: a normal measured load.
    #[default]
    #[serde(rename = "default")]
    Default,

    /// `sev:vmsa`: load the section as the BSP's 4 KiB VMSA page.
    #[serde(rename = "sev:vmsa")]
    Vmsa,
}

impl LoadKind {
    // `&self` is required by serde's `skip_serializing_if`.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_default(&self) -> bool {
        matches!(self, LoadKind::Default)
    }
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

/// `fill` action kinds accepted on the `sev` target: the core `dtb`,
/// plus `sev:secrets` and `sev:cpuid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillKind {
    /// Core `dtb` kind.
    #[serde(rename = "dtb")]
    Dtb,

    /// `sev:secrets`: a SEV-SNP secrets page.
    #[serde(rename = "sev:secrets")]
    Secrets,

    /// `sev:cpuid`: a SEV-SNP CPUID page.
    #[serde(rename = "sev:cpuid")]
    Cpuid,
}
