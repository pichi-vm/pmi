//! `cpu` extension: portable declaration of the vCPU ISA baseline.
//!
//! `cpu:profile` names an ISA baseline drawn from a per-architecture upstream
//! spec — x86-64 microarchitecture levels per the System V x86-64 psABI on
//! x86-64, or Armv8-A / Armv9-A revisions per the Arm Architecture Reference
//! Manual on aarch64. Profile values are not enumerated in PMI; the VMM
//! recognizes them by name and refuses to launch on values it does not know
//! or cannot satisfy. See `spec/cpu.md`.

use serde::{Deserialize, Serialize};

/// `cpu:profile` value: an ISA baseline name.
///
/// The crate carries the raw string faithfully. Validation (recognition,
/// host-capability check, the floor/measured-ceiling rule) is the VMM's
/// responsibility per `spec/cpu.md`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Profile(pub String);

impl Profile {
    /// Construct a `Profile` from any string-like input.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Borrow the profile name as `&str`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Profile {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<&str> for Profile {
    fn from(name: &str) -> Self {
        Self(name.to_owned())
    }
}
