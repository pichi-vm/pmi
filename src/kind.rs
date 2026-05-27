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
/// the `direct` and `merged` extensions, available on every target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillKind {
    /// `direct:dtb`: the section is populated with a host-supplied,
    /// unmeasured flattened devicetree blob (the entire platform definition).
    #[serde(rename = "direct:dtb")]
    DirectDtb,

    /// `merged:dtbo`: the section is populated with a host-supplied,
    /// unmeasured flattened devicetree overlay (resource allocation only;
    /// merged onto a measured base DTB named by the `merged:dtb` target
    /// attribute).
    #[serde(rename = "merged:dtbo")]
    MergedDtbo,
}
