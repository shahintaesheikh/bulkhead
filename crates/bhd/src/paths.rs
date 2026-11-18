//! Filesystem roots for `bhd`.
//!
//! Mirrors `bh::paths::BulkheadPaths`; the two binaries share the same layout
//! but not a crate. The duplication is small and flagged in the tracker.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bh_core::{Registry, RepoId};

/// The roots `bhd` resolves once per process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// `~/.bulkhead`, or `$BULKHEAD_HOME`.
    pub home: PathBuf,
    /// `~/.pi/agent`, or `$BULKHEAD_AGENT_DIR`.
    pub global_agent: PathBuf,
}

impl Paths {
    /// Resolves the roots from the environment.
    pub fn discover() -> Result<Self> {
        let user_home = env::var_os("HOME").context("HOME is not set")?;
        let user_home = PathBuf::from(user_home);
        let home = match env::var_os("BULKHEAD_HOME") {
            Some(value) => PathBuf::from(value),
            None => user_home.join(".bulkhead"),
        };
        let global_agent = match env::var_os("BULKHEAD_AGENT_DIR") {
            Some(value) => PathBuf::from(value),
            None => user_home.join(".pi/agent"),
        };
        Ok(Self { home, global_agent })
    }

    /// `~/.bulkhead/registry.toml`.
    pub fn registry_file(&self) -> PathBuf {
        self.home.join("registry.toml")
    }

    /// `~/.bulkhead/run/`.
    pub fn run_dir(&self) -> PathBuf {
        self.home.join("run")
    }

    /// `~/.bulkhead/run/bhd.sock`.
    pub fn socket_file(&self) -> PathBuf {
        self.run_dir().join("bhd.sock")
    }

    /// `~/.bulkhead/run/bhd.pid`.
    pub fn pid_file(&self) -> PathBuf {
        self.run_dir().join("bhd.pid")
    }

    /// `~/.bulkhead/<repo-id>/`.
    pub fn repo_home(&self, repo: &RepoId) -> PathBuf {
        self.home.join(repo.as_str())
    }

    /// `~/.bulkhead/<repo-id>/state/`.
    pub fn state_dir(&self, repo: &RepoId) -> PathBuf {
        self.repo_home(repo).join("state")
    }

    /// `~/.bulkhead/<repo-id>/state/wakes.jsonl`.
    pub fn wakes_file(&self, repo: &RepoId) -> PathBuf {
        self.state_dir(repo).join("wakes.jsonl")
    }
}

/// Loads the registry, or an empty one when it does not exist.
pub fn load_registry(paths: &Paths) -> Result<Registry> {
    let path = paths.registry_file();
    if !path.exists() {
        return Ok(Registry::default());
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}
