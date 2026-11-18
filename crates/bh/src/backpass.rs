//! backpass adapter: per-repo training of committed context.
//!
//! The CLI shape is assumed because backpass is not installed while bulkhead is
//! built; tests drive `tests/fixtures/fake-backpass`. The contract:
//!
//! - `backpass train <repo-root>` trains from the repo's committed context.
//!
//! `BULKHEAD_BACKPASS_BIN` overrides the binary.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

/// A thin client over the `backpass` CLI.
#[derive(Debug, Clone)]
pub struct Backpass {
    bin: PathBuf,
    child_env: Vec<(String, String)>,
}

impl Backpass {
    /// Builds a client for a given binary.
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self {
            bin: bin.into(),
            child_env: Vec::new(),
        }
    }

    /// Builds the default client, honoring `BULKHEAD_BACKPASS_BIN`.
    pub fn from_env() -> Self {
        let bin = std::env::var_os("BULKHEAD_BACKPASS_BIN")
            .map_or_else(|| PathBuf::from("backpass"), PathBuf::from);
        Self::new(bin)
    }

    /// Adds an explicit variable to every child process (used by tests).
    pub fn with_child_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.child_env.push((key.into(), value.into()));
        self
    }

    /// Trains this repo's context.
    pub fn run(&self, repo_root: &Path) -> Result<()> {
        let status = Command::new(&self.bin)
            .arg("train")
            .arg(repo_root)
            .envs(self.child_env.iter().map(|(key, value)| (key, value)))
            .status()
            .with_context(|| format!("running backpass train in {}", repo_root.display()))?;
        if !status.success() {
            bail!("backpass train exited with {status}");
        }
        Ok(())
    }
}
