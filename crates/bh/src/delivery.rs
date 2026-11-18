//! Delivery: landing approved work through a repo's delivery mode.

use anyhow::{Context, Result, bail};
use bh_core::{DeliveryMode, RepoId, Task, TaskId};

use crate::ghaxi::GhAxi;
use crate::herdr::Herdr;
use crate::no_mistakes::NoMistakes;
use crate::paths::BulkheadPaths;
use crate::treehouse::Treehouse;
use crate::{fsx, registry, task};

/// The adapters a landing needs.
pub struct Landers<'a> {
    /// herdr, to close the crew tab.
    pub herdr: &'a Herdr,
    /// treehouse, to release the worktree.
    pub treehouse: &'a Treehouse,
    /// gh-axi, for direct PRs.
    pub ghaxi: &'a GhAxi,
    /// no-mistakes, for gated delivery.
    pub no_mistakes: &'a NoMistakes,
}

/// What a landing did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandReport {
    /// The task that was landed.
    pub task_id: TaskId,
    /// The delivery mode used.
    pub mode: DeliveryMode,
    /// PR URL, PR number or worktree path, depending on the mode.
    pub reference: String,
}

/// Lands a task, its dependencies first, then tears it down.
pub fn land(
    paths: &BulkheadPaths,
    landers: &Landers<'_>,
    repo: &RepoId,
    task_id: &TaskId,
    approved: bool,
) -> Result<LandReport> {
    land_inner(paths, landers, repo, task_id, approved, 0)
}

fn land_inner(
    paths: &BulkheadPaths,
    landers: &Landers<'_>,
    repo: &RepoId,
    task_id: &TaskId,
    approved: bool,
    depth: usize,
) -> Result<LandReport> {
    if depth > 8 {
        bail!("dependency chain for {task_id} is too deep");
    }

    let registry = registry::load(paths)?;
    registry
        .repos
        .get(repo)
        .with_context(|| format!("{repo} is not registered"))?;
    let config = crate::config::load(paths, repo)?;
    let mode = config.delivery;
    let yolo = config.yolo;
    let task = task::load_task(paths, repo, task_id)?;

    if !approved && !yolo && mode != DeliveryMode::Local {
        bail!(
            "landing {task_id} needs the captain's approval; pass --approve (or set yolo = true for this repo)"
        );
    }

    if let Some(dependency) = &task.depends_on {
        if dependency != task_id {
            land_inner(paths, landers, repo, dependency, approved, depth + 1)?;
        }
    }

    let reference = match mode {
        DeliveryMode::DirectPr => {
            let number = pr_number(&task)
                .with_context(|| format!("task {task_id} has no PR URL to merge"))?;
            landers.ghaxi.merge(&number, "squash")?;
            number
        }
        DeliveryMode::NoMistakes => {
            let url = task
                .pr_url
                .clone()
                .with_context(|| format!("task {task_id} has no PR URL to land"))?;
            landers.no_mistakes.land(&url)?;
            url
        }
        DeliveryMode::Local => task.worktree.display().to_string(),
    };

    landers.herdr.close_tab(&task.tab)?;
    landers.treehouse.release(&task.worktree)?;
    fsx::write_atomic(&task::task_dir(paths, repo, task_id).join("landed"), "")?;

    Ok(LandReport {
        task_id: task_id.clone(),
        mode,
        reference,
    })
}

/// Extracts the PR number from a task's PR URL.
pub fn pr_number(task: &Task) -> Option<String> {
    task.pr_url
        .as_deref()
        .map(|url| url.trim_end_matches('/'))
        .and_then(|url| url.rsplit('/').next())
        .filter(|segment| !segment.is_empty())
        .map(str::to_owned)
}
