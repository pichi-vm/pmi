// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! `sev` target: AMD SEV-SNP confidential virtual machines.

use flagset::FlagSet;
use serde::{Deserialize, Serialize};

use crate::{Target, Version};

pub use crate::kind::LoadKind;
pub use crate::x86_64::Profile;

/// `sev` target spec, carried in the `.pmi.sev` PE section.
///
/// The VMM builds the BSP's VMSA as the Boot vCPU section of `spec/sev.md`
/// defines.
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

    /// Effective vCPU ISA baseline; defaults to the architecture baseline.
    #[serde(
        rename = "cpu:profile",
        default,
        skip_serializing_if = "crate::IsDefault::is_default"
    )]
    pub cpu_profile: Profile,

    /// SEV-SNP features the BSP's VMSA enables beyond `SNPActive`
    /// (`sev:features` target attribute). Empty when absent.
    #[serde(
        rename = "sev:features",
        default,
        skip_serializing_if = "features::is_empty",
        deserialize_with = "features::deserialize"
    )]
    pub features: FlagSet<Feature>,

    /// Optional `dt:dtb` target attribute: PE section holding the bundled
    /// base DTB. Present means a base DTB is bundled in the image, consumed by
    /// a `load` or as the default source of a `dt:dtb` fill; absent means the
    /// base is distributed separately (detached mode).
    #[serde(rename = "dt:dtb", default, skip_serializing_if = "Option::is_none")]
    pub dt_dtb: Option<String>,
}

impl Target for Spec {
    const NAME: &'static str = "sev";
    const SECTION: &'static str = ".pmi.sev";
}

flagset::flags! {
    /// A `sev:features` bit: a SEV-SNP feature the BSP's VMSA enables beyond
    /// `SNPActive`. Each value is the feature's `SEV_FEATURES` bit.
    pub enum Feature: u64 {
        /// `DebugVirtualization` (`SEV_FEATURES` bit 5).
        DebugVirtualization = 1 << 5,

        /// `SecureTSC` (`SEV_FEATURES` bit 9).
        SecureTsc = 1 << 9,
    }
}

/// `sev:features` on the wire: `FlagSet`'s integer form, refusing bits the
/// spec does not offer.
mod features {
    use flagset::FlagSet;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer};

    use super::Feature;

    // `&FlagSet` is required by serde's `skip_serializing_if`.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(super) fn is_empty(set: &FlagSet<Feature>) -> bool {
        set.is_empty()
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<FlagSet<Feature>, D::Error> {
        FlagSet::new(u64::deserialize(deserializer)?)
            .map_err(|_| D::Error::custom("sev:features sets a bit the spec does not offer"))
    }
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

/// `fill` action kinds accepted on the `sev` target: the cross-target
/// `dt` and `lbc` kinds, plus `sev:secrets` and `sev:cpuid`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillKind {
    /// `dt:dtb`: the VMM populates the Zero section with the measured base
    /// DTB (bundled copy or substitute).
    #[serde(rename = "dt:dtb")]
    DtDtb,

    /// `dt:dtbo`: host-supplied, unmeasured DTBO overlay (merged onto the
    /// measured base DTB).
    #[serde(rename = "dt:dtbo")]
    DtDtbo,

    /// Native Linux bootconfig, measured over the complete filled reservation.
    #[serde(rename = "lbc:bootconfig")]
    LbcBootconfig,

    /// `sev:secrets`: a SEV-SNP secrets page.
    #[serde(rename = "sev:secrets")]
    Secrets,

    /// `sev:cpuid`: a SEV-SNP CPUID page.
    #[serde(rename = "sev:cpuid")]
    Cpuid,
}
