//! Per-repo `config.toml` for `bhd`.

use std::fs;

use bh_core::{RepoConfig, RepoId};

use crate::paths::Paths;

/// Loads a repo's config, or defaults when missing or malformed.
pub fn load(paths: &Paths, repo: &RepoId) -> RepoConfig {
    let path = paths.repo_home(repo).join("config.toml");
    fs::read_to_string(&path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}
