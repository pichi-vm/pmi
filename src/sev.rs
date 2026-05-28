//! `sev` target: AMD SEV-SNP confidential virtual machines.

use serde::{Deserialize, Serialize};

use crate::{Target, Version};

pub use crate::cpu::Profile;

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

/// `fill` action kinds accepted on the `sev` target: the cross-target
/// `merged:dtbo`, plus `sev:secrets` and `sev:cpuid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillKind {
    /// `merged:dtbo`: host-supplied DTBO overlay (merged onto a measured
    /// base DTB named by the `merged:dtb` target attribute).
    #[serde(rename = "merged:dtbo")]
    MergedDtbo,

    /// `sev:secrets`: a SEV-SNP secrets page.
    #[serde(rename = "sev:secrets")]
    Secrets,

    /// `sev:cpuid`: a SEV-SNP CPUID page.
    #[serde(rename = "sev:cpuid")]
    Cpuid,
}
