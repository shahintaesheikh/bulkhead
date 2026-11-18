//! Creating the repo home directory layout under `~/.bulkhead/<repo-id>/`.

use std::fs;

use anyhow::{Context, Result};
use bh_core::{RepoConfig, RepoId};

use crate::fsx::write_if_missing;
use crate::paths::BulkheadPaths;

/// Directories created for every repo, relative to its home.
pub const REPO_DIRS: &[&str] = &[
    "agent",
    "agent/skills",
    "agent/extensions",
    "agent/prompts",
    "plans/inbox",
    "plans/accepted",
    "plans/declined",
    "backlog",
    "tasks",
    "state",
];

/// Creates the shared roots and one repo's home.
///
/// Idempotent: directories are created if absent and seed files are written
/// only when missing, so captain edits survive a re-run.
pub fn create(paths: &BulkheadPaths, repo: &RepoId) -> Result<()> {
    create_dir(&paths.shared_dir())?;
    create_dir(&paths.run_dir())?;

    let home = paths.repo_home(repo);
    for dir in REPO_DIRS {
        create_dir(&home.join(dir))?;
    }

    let config = toml::to_string_pretty(&RepoConfig::default()).context("serializing config")?;
    write_if_missing(&home.join("config.toml"), &config)?;
    write_if_missing(&home.join("orchestrator.md"), "")?;
    write_if_missing(&home.join("learnings.md"), "")?;

    Ok(())
}

fn create_dir(path: &std::path::Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("creating {}", path.display()))
}
