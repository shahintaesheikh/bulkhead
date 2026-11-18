//! Deterministic time and duration value types.
//!
//! `bh-core` never reads the clock itself; callers pass a [`Timestamp`] in.
//! That keeps classification and stall checks reproducible in tests.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A Unix timestamp in whole seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Builds a timestamp from Unix seconds.
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs)
    }

    /// Returns Unix seconds.
    pub const fn as_secs(self) -> u64 {
        self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A whole-seconds duration. Serialized as a plain integer in TOML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seconds(u64);

impl Seconds {
    /// Builds a duration from whole seconds.
    pub const fn from_secs(secs: u64) -> Self {
        Self(secs)
    }

    /// Returns the duration in whole seconds.
    pub const fn as_secs(self) -> u64 {
        self.0
    }
}

impl fmt::Display for Seconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}
