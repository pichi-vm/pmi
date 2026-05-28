//! Type definitions for the Portable Machine Image (PMI) spec.
//!
//! The normative spec lives under `spec/` in this repo. These types mirror
//! the per-target CBOR schemas defined there; the crate carries no PE I/O,
//! no FDT, no measurement, no platform glue. Implementations layer those
//! on top.

#![forbid(unsafe_code)]
#![deny(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub,
    trivial_casts,
    trivial_numeric_casts
)]
#![warn(clippy::all, clippy::pedantic)]

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

pub mod cca;
pub mod cpu;
pub mod sev;
pub mod tdx;
pub mod vm;

mod kind;

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

/// Schema version field pinned to `N`.
///
/// Serializes as `N`; deserializes only when the wire value equals `N`,
/// otherwise yields an "unsupported version" error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version<const N: u32>(());

impl<const N: u32> Default for Version<N> {
    fn default() -> Self {
        Self(())
    }
}

impl<const N: u32> Serialize for Version<N> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(N)
    }
}

impl<'de, const N: u32> Deserialize<'de> for Version<N> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = u32::deserialize(d)?;

        if v != N {
            return Err(de::Error::custom(format_args!(
                "unsupported version: expected {N}, got {v}"
            )));
        }

        Ok(Self::default())
    }
}
