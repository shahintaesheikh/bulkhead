//! Inputs the core loop consumes, produced by I/O-bearing event sources.
//!
//! Each event source in `bhd` sends exactly one shape of [`Event`] over a
//! channel. The core loop is the only owner of mutable state (architecture
//! invariant 2); sources never share a lock.

use crate::ids::{PaneId, PlanId, RepoId, TaskId, WakeId};
use crate::pane::PaneState;
use crate::status::StatusVerb;
use crate::time::Timestamp;

/// A supervision input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A crewmate appended a status line to its task's `status.log`.
    StatusLine {
        repo: RepoId,
        task: TaskId,
        verb: StatusVerb,
        message: String,
        /// When the line was observed. `bh-core` never reads the clock.
        at: Timestamp,
    },
    /// herdr reported a pane's agent state changed.
    PaneStateChanged {
        repo: RepoId,
        pane: PaneId,
        state: PaneState,
    },
    /// A new plan landed in a repo inbox.
    PlanArrived {
        repo: RepoId,
        plan: PlanId,
        /// When the plan was observed.
        at: Timestamp,
    },
    /// Away-mode toggled for a repo.
    AwayChanged { repo: RepoId, away: bool },
    /// A repo's orchestrator was found missing and is being restarted.
    OrchestratorGone {
        repo: RepoId,
        /// When the daemon noticed.
        at: Timestamp,
    },
    /// The extension acknowledged delivery of a wake.
    WakeAck { wake: WakeId },
    /// Periodic tick used for stall checks and wake batching.
    Tick,
    /// Drain the batcher, flush `wakes.jsonl` and stop.
    Shutdown,
}
