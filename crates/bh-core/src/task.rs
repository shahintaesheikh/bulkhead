//! Crew task metadata: the typed form of `tasks/<task-id>/meta.toml`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::{PaneId, RepoId, SessionRef, TabId, TaskId};
use crate::time::Timestamp;

/// Metadata recorded when a crew task is spawned.
///
/// This carries only facts `bh` records at spawn and teardown time. Work state
/// (working/blocked/done) is derived from the status log by the classifier, not
/// stored here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// Task id, also the task folder name.
    pub id: TaskId,
    /// Owning repo.
    pub repo: RepoId,
    /// Short human title from the backlog item.
    pub title: String,
    /// Absolute path to the treehouse worktree.
    pub worktree: PathBuf,
    /// herdr tab the crewmate runs in.
    pub tab: TabId,
    /// herdr pane the crewmate runs in.
    pub pane: PaneId,
    /// Pi session reference used to resume the crewmate.
    pub session: SessionRef,
    /// PR URL once delivery opens one.
    pub pr_url: Option<String>,
    /// Base branch for the PR, when this task is stacked.
    #[serde(default)]
    pub base: Option<String>,
    /// Task this one depends on; it is landed first.
    #[serde(default)]
    pub depends_on: Option<TaskId>,
    /// When the task was spawned.
    pub created_at: Timestamp,
    /// When the metadata last changed.
    pub updated_at: Timestamp,
}
