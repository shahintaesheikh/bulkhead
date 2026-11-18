//! Wakes: the durable, deduplicated unit of orchestrator attention.

use serde::{Deserialize, Serialize};

use crate::ids::{RepoId, TaskId, WakeId};
use crate::time::Timestamp;

/// Why the orchestrator is being woken.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WakeKind {
    /// A new plan landed in the repo inbox and needs triage.
    PlanInbox,
    /// A crewmate asked for a decision.
    NeedsDecision,
    /// A crewmate is blocked.
    Blocked,
    /// A crewmate opened or updated a PR.
    PrOpened,
    /// A crewmate reported `done`.
    TaskDone,
    /// A crewmate produced no status within the stall window.
    TaskStalled,
    /// A crewmate's pane or agent disappeared unexpectedly.
    CrewGone,
    /// `bhd` restarted an orchestrator that had stopped.
    OrchestratorRevived,
    /// Away-mode ended; the orchestrator owes the captain a return brief.
    AwayBrief,
}

impl WakeKind {
    /// A stable token for logs, digests and prompts.
    pub fn as_token(&self) -> &'static str {
        match self {
            Self::PlanInbox => "plan-inbox",
            Self::NeedsDecision => "needs-decision",
            Self::Blocked => "blocked",
            Self::PrOpened => "pr-opened",
            Self::TaskDone => "task-done",
            Self::TaskStalled => "task-stalled",
            Self::CrewGone => "crew-gone",
            Self::OrchestratorRevived => "orchestrator-revived",
            Self::AwayBrief => "away-brief",
        }
    }
}

/// One actionable event for an orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wake {
    /// Stable id the extension dedupes deliveries on.
    pub id: WakeId,
    /// The repo whose orchestrator is woken.
    pub repo: RepoId,
    /// The task this wake concerns, when it is task-scoped.
    pub task: Option<TaskId>,
    /// Why the orchestrator is being woken.
    pub kind: WakeKind,
    /// Rendered text delivered to the orchestrator.
    pub summary: String,
    /// When the wake was created.
    pub created_at: Timestamp,
}

/// One line of the append-only `wakes.jsonl`.
///
/// Delivery is recorded by appending [`WakeRecord::Delivered`] rather than
/// rewriting the wake, keeping the file append-only (invariant 3). A wake with
/// no matching delivered record is redelivered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "kebab-case")]
pub enum WakeRecord {
    /// A wake was created and must be delivered.
    Wake { wake: Wake },
    /// A wake was acknowledged by the extension.
    Delivered { id: WakeId, at: Timestamp },
}
