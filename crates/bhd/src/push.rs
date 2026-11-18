//! Phone push adapter.
//!
//! The push channel is an open question, so this is deliberately generic: if
//! `BULKHEAD_PUSH_BIN` is set, that program is run with the message as its only
//! argument. Any channel (ntfy, Pushover, a Slack webhook script) can sit
//! behind it. With it unset, pushes are disabled.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// A push channel behind a local command.
#[derive(Debug, Clone)]
pub struct Push {
    bin: PathBuf,
    child_env: Vec<(String, String)>,
}

impl Push {
    /// Builds a pusher around a binary.
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self {
            bin: bin.into(),
            child_env: Vec::new(),
        }
    }

    /// Builds a pusher when `BULKHEAD_PUSH_BIN` is set.
    pub fn from_env() -> Option<Self> {
        std::env::var_os("BULKHEAD_PUSH_BIN").map(Self::new)
    }

    /// Adds an explicit variable to every child process (used by tests).
    pub fn with_child_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.child_env.push((key.into(), value.into()));
        self
    }

    /// Sends one message.
    pub fn send(&self, message: &str) -> Result<()> {
        let status = Command::new(&self.bin)
            .arg(message)
            .envs(self.child_env.iter().map(|(key, value)| (key, value)))
            .status()
            .with_context(|| format!("running push {}", self.bin.display()))?;
        if !status.success() {
            bail!("push command exited with {status}");
        }
        Ok(())
    }
}
