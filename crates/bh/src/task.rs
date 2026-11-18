//! Crew task lifecycle: spawn, send, status and teardown.
//!
//! A task is a worktree, a task folder (`brief.md`, `status.log`, `meta.toml`)
//! and a herdr tab whose pane runs Pi with the repo's agent folder.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use bh_core::brief::BriefContext;
use bh_core::{DeliveryMode, PaneId, RepoId, SessionRef, TabId, Task, TaskId, Timestamp};

use crate::fsx;
use crate::herdr::Herdr;
use crate::paths::BulkheadPaths;
use crate::registry;
use crate::repo::DetectedRepo;
use crate::treehouse::Treehouse;

/// The crew brief template, versioned in `prompts/`.
pub const CREW_BRIEF: &str = include_str!("../../../prompts/crew-brief.md");

/// Result of a spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnReport {
    /// The spawned task.
    pub task_id: TaskId,
    /// Its worktree.
    pub worktree: PathBuf,
    /// Its herdr tab.
    pub tab: TabId,
    /// Its herdr pane.
    pub pane: PaneId,
}

/// Claims a worktree, renders the brief, and launches the crewmate.
pub fn spawn(
    paths: &BulkheadPaths,
    herdr: &Herdr,
    treehouse: &Treehouse,
    repo: &DetectedRepo,
    task_id: &TaskId,
) -> Result<SpawnReport> {
    let registry = registry::load(paths)?;
    let entry = registry
        .repos
        .get(&repo.id)
        .with_context(|| format!("{} is not registered", repo.id))?;
    let workspace = entry
        .workspace
        .clone()
        .context("repo has no herdr workspace; run `bh attach` first")?;
    let delivery = entry.delivery;

    let backlog = paths
        .repo_home(&repo.id)
        .join("backlog")
        .join(format!("{task_id}.md"));
    let raw = fs::read_to_string(&backlog).unwrap_or_default();
    let (title, body) = split_title_body(&raw);
    if title.is_empty() {
        bail!(
            "backlog item {task_id} has no title (expected a `#` heading in {})",
            backlog.display()
        );
    }

    let directory = task_dir(paths, &repo.id, task_id);
    fs::create_dir_all(&directory).with_context(|| format!("creating {}", directory.display()))?;

    let worktree = treehouse.claim(&repo.root, task_id)?;
    let status_log = directory.join("status.log");
    let brief_path = directory.join("brief.md");

    let context = BriefContext {
        repo: repo.id.to_string(),
        task_id: task_id.to_string(),
        title: title.clone(),
        worktree: worktree.display().to_string(),
        branch: branch_name(task_id),
        delivery: delivery_name(delivery).to_owned(),
        status_log: status_log.display().to_string(),
        boundaries: "see context/architecture.md".to_owned(),
        body,
    };
    fsx::write_atomic(&brief_path, &bh_core::render_brief(CREW_BRIEF, &context))?;
    fsx::write_if_missing(&status_log, "")?;

    let agent_dir = paths.agent_dir(&repo.id).to_string_lossy().into_owned();
    let env = [("PI_CODING_AGENT_DIR", agent_dir.as_str())];
    let created = herdr.create_tab(&workspace, &worktree, task_id.as_str(), &env)?;
    let pane = created.root_pane.pane_id.clone();

    let extra = [
        "--append-system-prompt".to_owned(),
        brief_path.to_string_lossy().into_owned(),
        "--session-id".to_owned(),
        task_id.to_string(),
    ];
    herdr.start_agent(&pane, task_id.as_str(), PI_KIND, &extra)?;

    let now = now();
    let task = Task {
        id: task_id.clone(),
        repo: repo.id.clone(),
        title,
        worktree: worktree.clone(),
        tab: created.tab.tab_id.clone(),
        pane: pane.clone(),
        session: SessionRef::from(task_id.as_str()),
        pr_url: None,
        created_at: now,
        updated_at: now,
    };
    write_task(&directory, &task)?;

    Ok(SpawnReport {
        task_id: task_id.clone(),
        worktree,
        tab: created.tab.tab_id,
        pane,
    })
}

/// Sends a message to a running crewmate.
pub fn send(
    herdr: &Herdr,
    paths: &BulkheadPaths,
    repo: &RepoId,
    task_id: &TaskId,
    message: &str,
) -> Result<()> {
    let task = load_task(paths, repo, task_id)?;
    herdr.prompt_agent(&task.pane, message)
}

/// Renders the status of one task, or lists the repo's tasks.
pub fn status(paths: &BulkheadPaths, repo: &RepoId, task_id: Option<&TaskId>) -> Result<String> {
    if let Some(task_id) = task_id {
        let task = load_task(paths, repo, task_id)?;
        Ok(render_status(&task, &status_tail(paths, repo, task_id)?))
    } else {
        let directory = paths.repo_home(repo).join("tasks");
        let mut output = String::new();
        if directory.is_dir() {
            let mut ids: Vec<String> = fs::read_dir(&directory)
                .with_context(|| format!("reading {}", directory.display()))?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            ids.sort();
            for id in ids {
                output.push_str(&id);
                output.push('\n');
            }
        }
        if output.is_empty() {
            output.push_str("(no tasks)\n");
        }
        Ok(output)
    }
}

/// Closes the crew pane and returns the worktree to the pool.
pub fn teardown(
    herdr: &Herdr,
    treehouse: &Treehouse,
    paths: &BulkheadPaths,
    repo: &RepoId,
    task_id: &TaskId,
) -> Result<()> {
    let task = load_task(paths, repo, task_id)?;
    herdr.close_tab(&task.tab)?;
    treehouse.release(&task.worktree)?;
    fsx::write_atomic(&task_dir(paths, repo, task_id).join("teardown"), "")?;
    Ok(())
}

/// Path to a task folder.
pub fn task_dir(paths: &BulkheadPaths, repo: &RepoId, task_id: &TaskId) -> PathBuf {
    paths.repo_home(repo).join("tasks").join(task_id.as_str())
}

/// Reads a task's `meta.toml`.
pub fn load_task(paths: &BulkheadPaths, repo: &RepoId, task_id: &TaskId) -> Result<Task> {
    let path = task_dir(paths, repo, task_id).join("meta.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn write_task(directory: &Path, task: &Task) -> Result<()> {
    let text = toml::to_string_pretty(task).context("serializing task metadata")?;
    fsx::write_atomic(&directory.join("meta.toml"), &text)
}

fn status_tail(paths: &BulkheadPaths, repo: &RepoId, task_id: &TaskId) -> Result<Vec<String>> {
    let path = task_dir(paths, repo, task_id).join("status.log");
    let text = fs::read_to_string(&path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().collect();
    Ok(lines
        .iter()
        .rev()
        .take(10)
        .rev()
        .map(|line| (*line).to_owned())
        .collect())
}

fn render_status(task: &Task, tail: &[String]) -> String {
    let mut output = format!(
        "{} — {}\nworktree: {}\ntab: {}\npane: {}\npr: {}\n",
        task.id,
        task.title,
        task.worktree.display(),
        task.tab,
        task.pane,
        task.pr_url.as_deref().unwrap_or("-"),
    );
    if tail.is_empty() {
        output.push_str("status: (no entries)\n");
    } else {
        output.push_str("status:\n");
        for line in tail {
            output.push_str("  ");
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

fn split_title_body(raw: &str) -> (String, String) {
    let mut title = String::new();
    let mut body = String::new();
    for line in raw.lines() {
        if title.is_empty() && line.trim_start().starts_with('#') {
            title = line.trim_start_matches('#').trim().to_owned();
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    (title, body.trim().to_owned())
}

fn branch_name(task_id: &TaskId) -> String {
    format!("bh/{task_id}")
}

fn delivery_name(mode: DeliveryMode) -> &'static str {
    match mode {
        DeliveryMode::NoMistakes => "no-mistakes",
        DeliveryMode::DirectPr => "direct-pr",
        DeliveryMode::Local => "local",
    }
}

fn now() -> Timestamp {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    Timestamp::from_secs(secs)
}

const PI_KIND: &str = "pi";
