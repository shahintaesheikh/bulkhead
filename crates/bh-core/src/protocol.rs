//! The versioned `bhd` <-> `bulkhead-orch` socket protocol.
//!
//! Frames are newline-delimited JSON envelopes. Both sides check the major
//! version and refuse an incompatible peer instead of guessing (invariant 10);
//! minor versions are additive only.

use serde::{Deserialize, Serialize};

use crate::ids::{RepoId, WakeId};
use crate::wake::Wake;

/// Major version of the `bhd` <-> extension protocol.
pub const PROTOCOL_MAJOR: u16 = 1;
/// Minor version. Bumped for additive changes only.
pub const PROTOCOL_MINOR: u16 = 0;

/// The protocol version a peer advertises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    /// Breaking-change counter.
    pub major: u16,
    /// Additive-change counter.
    pub minor: u16,
}

impl ProtocolVersion {
    /// The version this build speaks.
    pub const CURRENT: Self = Self {
        major: PROTOCOL_MAJOR,
        minor: PROTOCOL_MINOR,
    };

    /// Whether this build can talk to a peer advertising this version.
    pub const fn is_compatible(self) -> bool {
        self.major == PROTOCOL_MAJOR
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

/// A versioned protocol frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    /// Version the sender speaks.
    pub version: ProtocolVersion,
    /// The message body.
    pub message: ProtocolMessage,
}

/// One protocol message, tagged by `type` in JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProtocolMessage {
    /// Extension -> daemon: register this orchestrator.
    Hello { repo: RepoId, pid: u32 },
    /// Daemon -> extension: deliver a wake.
    Wake { wake: Wake },
    /// Extension -> daemon: a wake was delivered.
    Ack { wake: WakeId },
    /// Either direction: something went wrong.
    Error { message: String },
}

/// Errors decoding or validating a protocol frame.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// The peer speaks a major version this build does not implement.
    #[error("peer speaks protocol major {found}, this build speaks {ours}")]
    UnsupportedVersion {
        /// Major version the peer advertised.
        found: u16,
        /// Major version this build implements.
        ours: u16,
    },
    /// The line was not a valid protocol frame.
    #[error("malformed protocol frame")]
    Malformed(#[from] serde_json::Error),
}

impl Envelope {
    /// Builds a frame at the current protocol version.
    pub fn new(message: ProtocolMessage) -> Self {
        Self {
            version: ProtocolVersion::CURRENT,
            message,
        }
    }

    /// Encodes this frame as one newline-terminated JSON line.
    pub fn encode(&self) -> Result<String, ProtocolError> {
        let mut line = serde_json::to_string(self)?;
        line.push('\n');
        Ok(line)
    }

    /// Decodes one JSON line, refusing an unknown major version.
    pub fn decode(line: &str) -> Result<Self, ProtocolError> {
        let envelope: Self = serde_json::from_str(line.trim())?;
        let found = envelope.version.major;
        if !envelope.version.is_compatible() {
            return Err(ProtocolError::UnsupportedVersion {
                found,
                ours: PROTOCOL_MAJOR,
            });
        }
        Ok(envelope)
    }
}
