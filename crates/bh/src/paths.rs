//! Resolved filesystem roots.
//!
//! Nothing in `bh` reaches for the user's home on its own: `main` resolves a
//! [`BulkheadPaths`] once, and tests build one pointing at a temp directory.

use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bh_core::RepoId;

/// The two roots bulkhead resolves once per process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkheadPaths {
    /// `~/.bulkhead`, or `$BULKHEAD_HOME`.
    pub home: PathBuf,
    /// `~/.pi/agent`, or `$BULKHEAD_AGENT_DIR`; the source of linked globals.
    pub global_agent: PathBuf,
}

impl BulkheadPaths {
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

    /// `~/.bulkhead/shared/`.
    pub fn shared_dir(&self) -> PathBuf {
        self.home.join("shared")
    }

    /// `~/.bulkhead/run/`.
    pub fn run_dir(&self) -> PathBuf {
        self.home.join("run")
    }

    /// `~/.bulkhead/<repo-id>/`.
    pub fn repo_home(&self, repo: &RepoId) -> PathBuf {
        self.home.join(repo.as_str())
    }

    /// `~/.bulkhead/<repo-id>/agent/`: the repo's `PI_CODING_AGENT_DIR`.
    pub fn agent_dir(&self, repo: &RepoId) -> PathBuf {
        self.repo_home(repo).join("agent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_every_path_from_the_two_roots() {
        let paths = BulkheadPaths {
            home: PathBuf::from("/tmp/bh"),
            global_agent: PathBuf::from("/tmp/pi"),
        };
        let repo = RepoId::from("bulkhead");

        assert_eq!(
            paths.registry_file(),
            PathBuf::from("/tmp/bh/registry.toml")
        );
        assert_eq!(paths.repo_home(&repo), PathBuf::from("/tmp/bh/bulkhead"));
        assert_eq!(
            paths.agent_dir(&repo),
            PathBuf::from("/tmp/bh/bulkhead/agent")
        );
    }
}
