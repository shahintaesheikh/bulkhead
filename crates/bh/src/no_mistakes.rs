//! no-mistakes adapter: the PR gate for repos in no-mistakes delivery mode.
//!
//! The CLI shape is assumed because no-mistakes is not installed while
//! bulkhead is built; tests drive `tests/fixtures/fake-no-mistakes`. The
//! contract this adapter expects is:
//!
//! - `no-mistakes land <pr-url>` merges through the gate (exits non-zero on
//!   failure).
//!
//! `BULKHEAD_NOMISTAKES_BIN` overrides the binary.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// A thin client over the `no-mistakes` CLI.
#[derive(Debug, Clone)]
pub struct NoMistakes {
    bin: PathBuf,
    child_env: Vec<(String, String)>,
}

impl NoMistakes {
    /// Builds a client for a given binary.
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self {
            bin: bin.into(),
            child_env: Vec::new(),
        }
    }

    /// Builds the default client, honoring `BULKHEAD_NOMISTAKES_BIN`.
    pub fn from_env() -> Self {
        let bin = std::env::var_os("BULKHEAD_NOMISTAKES_BIN")
            .map_or_else(|| PathBuf::from("no-mistakes"), PathBuf::from);
        Self::new(bin)
    }

    /// Adds an explicit variable to every child process (used by tests).
    pub fn with_child_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.child_env.push((key.into(), value.into()));
        self
    }

    /// Lands a PR through the gate.
    pub fn land(&self, pr_url: &str) -> Result<()> {
        let mut command = Command::new(&self.bin);
        command
            .arg("land")
            .arg(pr_url)
            .envs(self.child_env.iter().map(|(key, value)| (key, value)));
        let output = command.output().context("running no-mistakes land")?;
        if !output.status.success() {
            bail!(
                "no-mistakes land failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }
}
