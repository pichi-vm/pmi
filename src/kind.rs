// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// `load` action kind discriminator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadKind {
    /// The core `default` kind; behavior is defined by the active target.
    #[default]
    Default,
}

impl LoadKind {
    // `&self` is required by serde's `skip_serializing_if`.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub(crate) fn is_default(&self) -> bool {
        matches!(self, LoadKind::Default)
    }
}

/// `fill` action kind discriminator: the cross-target fill kinds defined by
/// the `dt` extension, available on every target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillKind {
    /// `dt:dtb`: the section (a Zero section reserving the range) is populated
    /// by the VMM with the measured base DTB — the bundled copy named by the
    /// `dt:dtb` target attribute, or a substitute.
    #[serde(rename = "dt:dtb")]
    DtDtb,

    /// `dt:dtbo`: the section is populated with a host-supplied, unmeasured
    /// flattened devicetree overlay (resource allocation only; merged onto
    /// the measured base DTB).
    #[serde(rename = "dt:dtbo")]
    DtDtbo,
}
