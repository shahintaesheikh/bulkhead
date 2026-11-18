//! The `bh doctor` checks.
//!
//! Doctor only reports; it never mutates. Any [`Level::Error`] makes the CLI
//! exit non-zero.

use std::fs;
use std::path::Path;

use anyhow::Result;
use bh_core::{RegistryEntry, RepoConfig, RepoId};

use crate::agent_folder::LINKED;
use crate::paths::BulkheadPaths;
use crate::registry;

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// A check passed.
    Ok,
    /// Something is off but not broken.
    Warn,
    /// Something is broken.
    Error,
}

/// One result from [`doctor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Severity.
    pub level: Level,
    /// Repo the finding is about, if any.
    pub repo: Option<RepoId>,
    /// Human-readable description.
    pub message: String,
}

impl Finding {
    fn ok(repo: Option<&RepoId>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Ok,
            repo: repo.cloned(),
            message: message.into(),
        }
    }

    fn warn(repo: Option<&RepoId>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Warn,
            repo: repo.cloned(),
            message: message.into(),
        }
    }

    fn error(repo: Option<&RepoId>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            repo: repo.cloned(),
            message: message.into(),
        }
    }
}

/// Runs every check and returns all findings.
pub fn doctor(paths: &BulkheadPaths) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let registry = registry::load(paths)?;

    if registry.repos.is_empty() {
        findings.push(Finding::warn(
            None,
            format!("no repos registered in {}", paths.registry_file().display()),
        ));
        return Ok(findings);
    }

    for (repo, entry) in &registry.repos {
        check_repo(paths, repo, entry, &mut findings);
    }

    Ok(findings)
}

fn check_repo(
    paths: &BulkheadPaths,
    repo: &RepoId,
    entry: &RegistryEntry,
    findings: &mut Vec<Finding>,
) {
    let mut errors = 0usize;
    let home = paths.repo_home(repo);
    let agent = paths.agent_dir(repo);

    if !entry.path.is_dir() {
        findings.push(Finding::error(
            Some(repo),
            format!("repo path {} is not a directory", entry.path.display()),
        ));
        errors += 1;
    }
    if !home.is_dir() {
        findings.push(Finding::error(
            Some(repo),
            format!("home {} is missing", home.display()),
        ));
        errors += 1;
    }
    if !agent.is_dir() {
        findings.push(Finding::error(
            Some(repo),
            format!("agent dir {} is missing", agent.display()),
        ));
        return;
    }

    errors += check_config(&home.join("config.toml"), repo, findings);
    errors += check_json(&agent.join("settings.json"), repo, findings);
    errors += check_json(&agent.join("mcp.json"), repo, findings);
    errors += check_links(paths, &agent, repo, findings);

    if errors == 0 {
        findings.push(Finding::ok(Some(repo), format!("{repo} looks healthy")));
    }
}

fn check_config(path: &Path, repo: &RepoId, findings: &mut Vec<Finding>) -> usize {
    match fs::read_to_string(path) {
        Ok(text) => match toml::from_str::<RepoConfig>(&text) {
            Ok(_) => 0,
            Err(err) => {
                findings.push(Finding::error(
                    Some(repo),
                    format!("{} is invalid: {err}", path.display()),
                ));
                1
            }
        },
        Err(_) => {
            findings.push(Finding::error(
                Some(repo),
                format!("{} is missing", path.display()),
            ));
            1
        }
    }
}

fn check_json(path: &Path, repo: &RepoId, findings: &mut Vec<Finding>) -> usize {
    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(_) => 0,
            Err(err) => {
                findings.push(Finding::error(
                    Some(repo),
                    format!("{} is invalid JSON: {err}", path.display()),
                ));
                1
            }
        },
        Err(_) => {
            findings.push(Finding::error(
                Some(repo),
                format!("{} is missing", path.display()),
            ));
            1
        }
    }
}

fn check_links(
    paths: &BulkheadPaths,
    agent: &Path,
    repo: &RepoId,
    findings: &mut Vec<Finding>,
) -> usize {
    let mut errors = 0usize;

    for name in LINKED {
        let dest = agent.join(name);
        let meta = match fs::symlink_metadata(&dest) {
            Ok(meta) => meta,
            Err(_) => {
                if paths.global_agent.join(name).exists() {
                    findings.push(Finding::warn(
                        Some(repo),
                        format!("{name} is present globally but not linked here"),
                    ));
                }
                continue;
            }
        };

        if meta.file_type().is_symlink() {
            if fs::metadata(&dest).is_err() {
                findings.push(Finding::error(
                    Some(repo),
                    format!("broken symlink {}", dest.display()),
                ));
                errors += 1;
            }
        } else {
            findings.push(Finding::error(
                Some(repo),
                format!("stray {name}: {} is not a symlink", dest.display()),
            ));
            errors += 1;
        }
    }

    errors
}
