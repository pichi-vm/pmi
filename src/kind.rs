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

/// `fill` action kind discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FillKind {
    /// Core `dtb` kind: the section is populated with a host-supplied,
    /// unmeasured flattened devicetree blob.
    Dtb,
}
