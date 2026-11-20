// SPDX-FileCopyrightText: Advanced Micro Devices, Inc.
// SPDX-License-Identifier: Apache-2.0

//! x86-64: the architecture-specific parts of PMI (`spec/x86-64.md`).

use std::fmt;

use serde::{de, Deserialize, Deserializer, Serialize};

/// x86-64 `cpu:profile` value. Serializes as its name in the spec.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Profile {
    /// `x86-64-v1`, the psABI baseline.
    #[serde(rename = "x86-64-v1")]
    #[default]
    V1,
    /// `x86-64-v2`.
    #[serde(rename = "x86-64-v2")]
    V2,
    /// `x86-64-v3`.
    #[serde(rename = "x86-64-v3")]
    V3,
    /// `x86-64-v4`.
    #[serde(rename = "x86-64-v4")]
    V4,
}

impl<'de> Deserialize<'de> for Profile {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct ProfileName;

        impl de::Visitor<'_> for ProfileName {
            type Value = Profile;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an x86-64 CPU profile name")
            }

            fn visit_str<E: de::Error>(self, name: &str) -> Result<Profile, E> {
                match name {
                    "x86-64-v1" => Ok(Profile::V1),
                    "x86-64-v2" => Ok(Profile::V2),
                    "x86-64-v3" => Ok(Profile::V3),
                    "x86-64-v4" => Ok(Profile::V4),
                    _ => Err(E::invalid_value(de::Unexpected::Str(name), &self)),
                }
            }
        }

        // Preserve CBOR's wire type, including support for chunked text.
        d.deserialize_any(ProfileName)
    }
}
