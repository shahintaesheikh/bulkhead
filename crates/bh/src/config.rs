//! Per-repo `config.toml` load and save.

use std::fs;

use anyhow::{Context, Result};
use bh_core::{RepoConfig, RepoId};

use crate::fsx::write_atomic;
use crate::paths::BulkheadPaths;

/// Loads a repo's config, or defaults when the file does not exist.
pub fn load(paths: &BulkheadPaths, repo: &RepoId) -> Result<RepoConfig> {
    let path = paths.repo_home(repo).join("config.toml");
    if !path.exists() {
        return Ok(RepoConfig::default());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Writes a repo's config atomically.
pub fn save(paths: &BulkheadPaths, repo: &RepoId, config: &RepoConfig) -> Result<()> {
    let text = toml::to_string_pretty(config).context("serializing config")?;
    write_atomic(&paths.repo_home(repo).join("config.toml"), &text)
}
