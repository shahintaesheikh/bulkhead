//! Treehouse adapter: pooled task worktrees.
//!
//! The CLI shape is assumed because treehouse is not installed while bulkhead
//! is being built; tests drive `tests/fixtures/fake-treehouse`. The contract
//! this adapter expects is:
//!
//! - `treehouse claim <repo-root> <task-id>` prints the absolute worktree path.
//! - `treehouse release <worktree>` returns it to the pool.
//!
//! `BULKHEAD_TREEHOUSE_BIN` overrides the binary.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use bh_core::TaskId;

/// A thin client over the `treehouse` CLI.
#[derive(Debug, Clone)]
pub struct Treehouse {
    bin: PathBuf,
    child_env: Vec<(String, String)>,
}

impl Treehouse {
    /// Builds a client for a given binary.
    pub fn new(bin: impl Into<PathBuf>) -> Self {
        Self {
            bin: bin.into(),
            child_env: Vec::new(),
        }
    }

    /// Adds an explicit variable to every child process (used by tests).
    pub fn with_child_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.child_env.push((key.into(), value.into()));
        self
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.bin);
        command.envs(self.child_env.iter().map(|(key, value)| (key, value)));
        command
    }

    /// Builds the default client, honoring `BULKHEAD_TREEHOUSE_BIN`.
    pub fn from_env() -> Self {
        let bin = std::env::var_os("BULKHEAD_TREEHOUSE_BIN")
            .map_or_else(|| PathBuf::from("treehouse"), PathBuf::from);
        Self::new(bin)
    }

    /// Claims a worktree for a task and returns its absolute path.
    pub fn claim(&self, repo_root: &Path, task: &TaskId) -> Result<PathBuf> {
        let output = self
            .command()
            .arg("claim")
            .arg(repo_root)
            .arg(task.as_str())
            .output()
            .context("running treehouse claim")?;

        if !output.status.success() {
            bail!(
                "treehouse claim failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if path.is_empty() {
            bail!("treehouse claim returned no worktree path");
        }
        Ok(PathBuf::from(path))
    }

    /// Releases a worktree back to the pool.
    pub fn release(&self, worktree: &Path) -> Result<()> {
        let output = self
            .command()
            .arg("release")
            .arg(worktree)
            .output()
            .context("running treehouse release")?;

        if !output.status.success() {
            bail!(
                "treehouse release failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }
}
