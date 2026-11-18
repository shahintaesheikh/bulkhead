//! The pure supervision state machine.
//!
//! [`classify`] is the only place that decides whether an event deserves an
//! orchestrator wake. It is a pure transition: the whole state goes in with one
//! event, and the next state plus any wakes come out. No clock, files or
//! processes are touched, so every rule is testable without a runtime.

use std::collections::{BTreeMap, BTreeSet};

use crate::event::Event;
use crate::ids::{PaneId, PlanId, RepoId, TaskId, WakeId};
use crate::pane::PaneState;
use crate::status::StatusVerb;
use crate::time::Seconds;
use crate::time::Timestamp;
use crate::wake::{Wake, WakeKind};

/// All supervision state the core loop owns.
///
/// Wake ids come from a monotonic counter in the state, so classification stays
/// deterministic. `bhd` seeds the counter from `wakes.jsonl` on restart.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SupervisionState {
    repos: BTreeMap<RepoId, RepoState>,
    next_wake_seq: u64,
}

impl SupervisionState {
    /// An empty state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Read-only view of one repo's state.
    pub fn repo(&self, repo: &RepoId) -> Option<&RepoState> {
        self.repos.get(repo)
    }

    /// Every repo's state.
    pub fn repos(&self) -> &BTreeMap<RepoId, RepoState> {
        &self.repos
    }

    /// How many wakes have been issued.
    pub fn issued_wakes(&self) -> u64 {
        self.next_wake_seq
    }

    /// Seeds the wake counter when rehydrating from `wakes.jsonl`.
    pub fn seed_wake_seq(&mut self, seq: u64) {
        self.next_wake_seq = seq;
    }

    fn ensure_repo(&mut self, repo: &RepoId) -> &mut RepoState {
        self.repos.entry(repo.clone()).or_default()
    }

    fn issue_wake(
        &mut self,
        repo: &RepoId,
        task: Option<&TaskId>,
        kind: WakeKind,
        summary: String,
        at: Timestamp,
    ) -> Wake {
        self.next_wake_seq += 1;
        Wake {
            id: WakeId::new(format!("w-{}", self.next_wake_seq)),
            repo: repo.clone(),
            task: task.cloned(),
            kind,
            summary,
            created_at: at,
        }
    }
}

/// Supervision state for one repo.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RepoState {
    /// Away-mode is on for this repo.
    pub away: bool,
    /// Per-task state, keyed by task id.
    pub tasks: BTreeMap<TaskId, TaskState>,
    /// Last known herdr state per pane.
    pub panes: BTreeMap<PaneId, PaneState>,
    /// Plans seen in the inbox but not yet triaged.
    pub pending_plans: BTreeSet<PlanId>,
}

/// Task state derived from status lines.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaskState {
    /// The most recent status verb.
    pub last_verb: Option<StatusVerb>,
    /// When the most recent status line was observed.
    pub last_status_at: Option<Timestamp>,
    /// Whether a stall wake has already been issued for this silence.
    pub stalled: bool,
}

/// Applies one event and returns the next state plus any wakes it produced.
pub fn classify(mut state: SupervisionState, event: Event) -> (SupervisionState, Vec<Wake>) {
    let mut wakes = Vec::new();

    match event {
        Event::StatusLine {
            repo,
            task,
            verb,
            message,
            at,
        } => {
            {
                let task_state = state
                    .ensure_repo(&repo)
                    .tasks
                    .entry(task.clone())
                    .or_default();
                task_state.last_verb = Some(verb.clone());
                task_state.last_status_at = Some(at);
                task_state.stalled = false;
            }
            if let Some(kind) = wake_kind(&verb) {
                let summary = format!("{task}: {} — {message}", verb.as_token());
                wakes.push(state.issue_wake(&repo, Some(&task), kind, summary, at));
            }
        }
        Event::PaneStateChanged {
            repo,
            pane,
            state: pane_state,
        } => {
            state.ensure_repo(&repo).panes.insert(pane, pane_state);
        }
        Event::PlanArrived { repo, plan, at } => {
            let inserted = state.ensure_repo(&repo).pending_plans.insert(plan.clone());
            if inserted {
                let summary = format!("plan {plan} arrived");
                wakes.push(state.issue_wake(&repo, None, WakeKind::PlanInbox, summary, at));
            }
        }
        Event::AwayChanged { repo, away } => {
            state.ensure_repo(&repo).away = away;
        }
        Event::OrchestratorGone { repo, at } => {
            state.ensure_repo(&repo);
            wakes.push(state.issue_wake(
                &repo,
                None,
                WakeKind::OrchestratorRevived,
                "orchestrator was down and has been restarted".to_owned(),
                at,
            ));
        }
        Event::WakeAck { .. } | Event::Tick | Event::Shutdown => {}
    }

    (state, wakes)
}

/// Emits at most one `TaskStalled` wake per silent window, per task.
///
/// A task is stalled when its last status is older than its repo's threshold
/// and was not terminal. A new status line clears the flag.
pub fn check_stalls(
    mut state: SupervisionState,
    now: Timestamp,
    stall_after: &BTreeMap<RepoId, Seconds>,
) -> (SupervisionState, Vec<Wake>) {
    let mut stalled: Vec<(RepoId, TaskId, u64)> = Vec::new();

    for (repo, repo_state) in state.repos.iter_mut() {
        let Some(threshold) = stall_after.get(repo) else {
            continue;
        };
        for (task, task_state) in repo_state.tasks.iter_mut() {
            if task_state.stalled {
                continue;
            }
            let Some(last) = task_state.last_status_at else {
                continue;
            };
            if task_state
                .last_verb
                .as_ref()
                .is_some_and(StatusVerb::is_terminal)
            {
                continue;
            }
            let silent = now.as_secs().saturating_sub(last.as_secs());
            if silent < threshold.as_secs() {
                continue;
            }
            task_state.stalled = true;
            stalled.push((repo.clone(), task.clone(), threshold.as_secs()));
        }
    }

    let mut wakes = Vec::new();
    for (repo, task, seconds) in stalled {
        let summary = format!("{task} stalled: no status for {seconds}s");
        wakes.push(state.issue_wake(&repo, Some(&task), WakeKind::TaskStalled, summary, now));
    }
    (state, wakes)
}

/// The wake an actionable verb produces, if any.
fn wake_kind(verb: &StatusVerb) -> Option<WakeKind> {
    match verb {
        StatusVerb::NeedsDecision => Some(WakeKind::NeedsDecision),
        StatusVerb::Blocked => Some(WakeKind::Blocked),
        StatusVerb::Pr => Some(WakeKind::PrOpened),
        StatusVerb::Done => Some(WakeKind::TaskDone),
        StatusVerb::Working | StatusVerb::Unknown(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> RepoId {
        RepoId::from("bulkhead")
    }

    fn task() -> TaskId {
        TaskId::from("t-1")
    }

    fn at() -> Timestamp {
        Timestamp::from_secs(1_700_000_000)
    }

    fn status(verb: StatusVerb, message: &str) -> Event {
        status_at(verb, message, at())
    }

    fn status_at(verb: StatusVerb, message: &str, at: Timestamp) -> Event {
        Event::StatusLine {
            repo: repo(),
            task: task(),
            verb,
            message: message.to_owned(),
            at,
        }
    }

    #[test]
    fn stalls_once_per_silent_window() {
        let mut thresholds = BTreeMap::new();
        thresholds.insert(repo(), Seconds::from_secs(100));

        let (mut state, _) = classify(
            SupervisionState::new(),
            status_at(StatusVerb::Working, "go", Timestamp::from_secs(1_000)),
        );

        let (next, wakes) = check_stalls(state, Timestamp::from_secs(1_150), &thresholds);
        state = next;
        assert_eq!(wakes.len(), 1);
        assert_eq!(wakes[0].kind, WakeKind::TaskStalled);

        let (next, wakes) = check_stalls(state, Timestamp::from_secs(1_400), &thresholds);
        state = next;
        assert!(wakes.is_empty(), "a stall is reported once");

        let (next, _) = classify(
            state,
            status_at(StatusVerb::Working, "again", Timestamp::from_secs(1_450)),
        );
        state = next;
        let (_, wakes) = check_stalls(state, Timestamp::from_secs(1_600), &thresholds);
        assert_eq!(wakes.len(), 1, "a new status re-arms the stall check");
    }

    #[test]
    fn working_and_unknown_do_not_wake() {
        for verb in [
            StatusVerb::Working,
            StatusVerb::Unknown("deploying".to_owned()),
        ] {
            let (state, wakes) =
                classify(SupervisionState::new(), status(verb.clone(), "progress"));
            assert!(wakes.is_empty(), "{verb:?} should not wake");
            assert_eq!(
                state.repo(&repo()).unwrap().tasks[&task()].last_verb,
                Some(verb)
            );
        }
    }

    #[test]
    fn every_actionable_verb_wakes_with_expected_kind() {
        let cases = [
            (StatusVerb::NeedsDecision, WakeKind::NeedsDecision),
            (StatusVerb::Blocked, WakeKind::Blocked),
            (StatusVerb::Pr, WakeKind::PrOpened),
            (StatusVerb::Done, WakeKind::TaskDone),
        ];

        let mut state = SupervisionState::new();
        for (index, (verb, kind)) in cases.into_iter().enumerate() {
            let (next, wakes) = classify(state, status(verb, "message"));
            state = next;
            assert_eq!(wakes.len(), 1);
            assert_eq!(wakes[0].kind, kind);
            assert_eq!(wakes[0].task.as_ref(), Some(&task()));
            assert_eq!(wakes[0].id, WakeId::from(format!("w-{}", index + 1)));
            assert_eq!(wakes[0].created_at, at());
            assert_eq!(wakes[0].repo, repo());
        }
    }

    #[test]
    fn status_line_updates_task_state() {
        let (state, _) = classify(SupervisionState::new(), status(StatusVerb::Working, "hi"));
        let task_state = &state.repo(&repo()).unwrap().tasks[&task()];
        assert_eq!(task_state.last_verb, Some(StatusVerb::Working));
        assert_eq!(task_state.last_status_at, Some(at()));
    }

    #[test]
    fn repeated_plan_arrival_wakes_only_while_pending() {
        let plan = PlanId::from("p-1");
        let event = || Event::PlanArrived {
            repo: repo(),
            plan: plan.clone(),
            at: at(),
        };

        let (state, wakes) = classify(SupervisionState::new(), event());
        assert_eq!(wakes.len(), 1);
        assert_eq!(wakes[0].kind, WakeKind::PlanInbox);
        assert_eq!(wakes[0].task, None);

        let (state, wakes) = classify(state, event());
        assert!(wakes.is_empty(), "a pending plan must not wake twice");
        assert!(state.repo(&repo()).unwrap().pending_plans.contains(&plan));
    }

    #[test]
    fn pane_state_and_away_are_recorded_without_wakes() {
        let (state, wakes) = classify(
            SupervisionState::new(),
            Event::PaneStateChanged {
                repo: repo(),
                pane: PaneId::from("w1:p1"),
                state: PaneState::Blocked,
            },
        );
        assert!(wakes.is_empty());
        assert_eq!(
            state.repo(&repo()).unwrap().panes[&PaneId::from("w1:p1")],
            PaneState::Blocked
        );

        let (state, wakes) = classify(
            state,
            Event::AwayChanged {
                repo: repo(),
                away: true,
            },
        );
        assert!(wakes.is_empty());
        assert!(state.repo(&repo()).unwrap().away);
    }

    #[test]
    fn wake_ids_are_monotonic_across_events() {
        let mut state = SupervisionState::new();
        let mut ids = Vec::new();
        for _ in 0..3 {
            let (next, wakes) = classify(state, status(StatusVerb::Done, "done"));
            state = next;
            ids.push(wakes[0].id.clone());
        }
        assert_eq!(
            ids,
            vec![
                WakeId::from("w-1"),
                WakeId::from("w-2"),
                WakeId::from("w-3")
            ]
        );
        assert_eq!(state.issued_wakes(), 3);
    }
}
