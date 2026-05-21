//! Type definitions for the Portable Machine Image (PMI) spec.
//!
//! The normative spec lives under `spec/` in this repo. These types mirror
//! the per-target CBOR schemas defined there; the crate carries no PE I/O,
//! no FDT, no measurement, no platform glue. Implementations layer those
//! on top.

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

pub mod cca;
pub mod sev;
pub mod tdx;
pub mod vcpu;
pub mod vm;

/// Common interface for a top-level PMI target spec.
///
/// Each target's CBOR spec lives in its own PE section named by
/// [`Target::SECTION`].
pub trait Target: Serialize + de::DeserializeOwned {
    /// The target's short name (e.g., `"vm"`).
    const NAME: &'static str;
    /// The PE section name carrying this target's spec (e.g., `".pmi.vm"`).
    const SECTION: &'static str;
}

/// Schema version field with the version number baked into the type.
///
/// Serializes as the const generic `N`; deserialization succeeds only when
/// the wire value equals `N`. Any other value yields an "unsupported
/// version" error — the spec's "MUST reject sections with an unrecognized
/// version" rule expressed in the type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version<const N: usize>(());

impl<const N: usize> Version<N> {
    pub const fn new() -> Self {
        Self(())
    }
}

impl<const N: usize> Default for Version<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Serialize for Version<N> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(N as u64)
    }
}

impl<'de, const N: usize> Deserialize<'de> for Version<N> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u64::deserialize(d)?;
        if v == N as u64 {
            Ok(Self::new())
        } else {
            Err(de::Error::custom(format_args!(
                "unsupported version: expected {}, got {}",
                N, v
            )))
        }
    }
}
