//! `bh ls`: a read-only summary of every registered repo.

use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use bh_core::{Registry, RepoId, WorkspaceId};

use crate::herdr::WorkspaceInfo;
use crate::paths::BulkheadPaths;

/// One row of `bh ls`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSummary {
    /// Repo id.
    pub id: RepoId,
    /// Primary checkout path.
    pub path: PathBuf,
    /// Bound herdr workspace, if attached.
    pub workspace: Option<WorkspaceId>,
    /// That workspace's label, if the daemon knows it.
    pub workspace_label: Option<String>,
    /// Number of task folders.
    pub tasks: usize,
}

/// Summarizes the registry, matching workspaces by id.
pub fn summarize(
    paths: &BulkheadPaths,
    registry: &Registry,
    workspaces: &[WorkspaceInfo],
) -> Result<Vec<RepoSummary>> {
    let mut rows = Vec::new();
    for (repo, entry) in &registry.repos {
        let label = entry
            .workspace
            .as_ref()
            .and_then(|id| {
                workspaces
                    .iter()
                    .find(|workspace| &workspace.workspace_id == id)
            })
            .map(|workspace| workspace.label.clone());
        rows.push(RepoSummary {
            id: repo.clone(),
            path: entry.path.clone(),
            workspace: entry.workspace.clone(),
            workspace_label: label,
            tasks: count_tasks(paths, repo),
        });
    }
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(rows)
}

fn count_tasks(paths: &BulkheadPaths, repo: &RepoId) -> usize {
    let directory = paths.repo_home(repo).join("tasks");
    fs::read_dir(directory)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .count()
        })
        .unwrap_or(0)
}
