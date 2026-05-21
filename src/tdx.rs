use serde::{Deserialize, Serialize};

use crate::{Target, Version};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub version: Version<1>,
    pub dtb: String,
    pub actions: Vec<Action>,
}

impl Target for Spec {
    const NAME: &'static str = "tdx";
    const SECTION: &'static str = ".pmi.tdx";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    Load(Load),
    Fill(Fill),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Load {
    pub section: String,
    #[serde(default, skip_serializing_if = "LoadKind::is_default")]
    pub kind: LoadKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadKind {
    #[default]
    Measured,
    Unmeasured,
}

impl LoadKind {
    fn is_default(&self) -> bool {
        matches!(self, LoadKind::Measured)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    pub section: String,
    pub kind: FillKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FillKind {
    Dtbo,
}
