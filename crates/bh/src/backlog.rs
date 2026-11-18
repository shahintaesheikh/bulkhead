//! Backlog items: `<repo-home>/backlog/<task-id>.md`.
//!
//! Each item is frontmatter (`title`, `state`) plus the ask as the body.
//! `bh spawn` turns one into a task.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bh_core::frontmatter;
use bh_core::{RepoId, TaskId};

use crate::fsx;
use crate::paths::BulkheadPaths;
use crate::repo::slug_text;

/// One backlog item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacklogItem {
    /// Task id the item will spawn as.
    pub id: TaskId,
    /// Human title.
    pub title: String,
    /// State token (`backlog`, `doing`, ...).
    pub state: String,
    /// Absolute path to the item.
    pub path: PathBuf,
}

/// Creates a backlog item and returns its id.
pub fn create(paths: &BulkheadPaths, repo: &RepoId, ask: &str) -> Result<TaskId> {
    let title = ask
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("task")
        .trim()
        .to_owned();
    let base = slug_text(&title).unwrap_or_else(|| "task".to_owned());

    let directory = paths.repo_home(repo).join("backlog");
    fs::create_dir_all(&directory).with_context(|| format!("creating {}", directory.display()))?;

    let mut id = base.clone();
    let mut suffix = 2;
    while directory.join(format!("{id}.md")).exists() {
        id = format!("{base}-{suffix}");
        suffix += 1;
    }

    let body = format!("{ask}\n");
    let text = frontmatter::render(&[("title", &title), ("state", "backlog")], &body);
    fsx::write_atomic(&directory.join(format!("{id}.md")), &text)?;
    Ok(TaskId::new(id))
}

/// Lists the repo's backlog items.
pub fn list(paths: &BulkheadPaths, repo: &RepoId) -> Result<Vec<BacklogItem>> {
    let directory = paths.repo_home(repo).join("backlog");
    let mut items = Vec::new();
    if !directory.is_dir() {
        return Ok(items);
    }

    for entry in
        fs::read_dir(&directory).with_context(|| format!("reading {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        let (fields, _) = frontmatter::parse(&text).unwrap_or_default();
        let id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("task")
            .to_owned();
        items.push(BacklogItem {
            id: TaskId::new(id),
            title: fields.get("title").cloned().unwrap_or_default(),
            state: fields.get("state").cloned().unwrap_or_default(),
            path,
        });
    }

    items.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(items)
}
