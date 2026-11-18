//! Serde round-trip tests for every persisted or protocol type in Unit 01.
//!
//! TOML is exercised for on-disk config/metadata; JSON for wakes, records and
//! the socket protocol.

use std::collections::BTreeMap;
use std::path::PathBuf;

use bh_core::config::{DeliveryMode, Registry, RegistryEntry, RepoConfig};
use bh_core::ids::{PaneId, RepoId, SessionRef, TabId, TaskId, WakeId, WorkspaceId};
use bh_core::pane::PaneState;
use bh_core::protocol::{Envelope, PROTOCOL_MAJOR, ProtocolError, ProtocolMessage};
use bh_core::status::StatusVerb;
use bh_core::task::Task;
use bh_core::time::{Seconds, Timestamp};
use bh_core::wake::{Wake, WakeKind, WakeRecord};

fn sample_wake() -> Wake {
    Wake {
        id: WakeId::from("w-1"),
        repo: RepoId::from("bulkhead"),
        task: Some(TaskId::from("t-1")),
        kind: WakeKind::NeedsDecision,
        summary: "t-1 needs a decision: pick a socket format".to_owned(),
        created_at: Timestamp::from_secs(1_700_000_000),
    }
}

#[test]
fn status_verb_round_trips_through_json() {
    for verb in [
        StatusVerb::Working,
        StatusVerb::NeedsDecision,
        StatusVerb::Blocked,
        StatusVerb::Pr,
        StatusVerb::Done,
        StatusVerb::Unknown("waiting".to_owned()),
    ] {
        let json = serde_json::to_string(&verb).unwrap();
        let back: StatusVerb = serde_json::from_str(&json).unwrap();
        assert_eq!(verb, back);
    }
}

#[test]
fn pane_state_round_trips_through_json() {
    for state in [
        PaneState::Idle,
        PaneState::Working,
        PaneState::Blocked,
        PaneState::Done,
        PaneState::Unknown,
    ] {
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, format!("\"{}\"", state.as_token()));
        let back: PaneState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, back);
    }
}

#[test]
fn wake_and_records_round_trip_through_json() {
    let wake = sample_wake();

    let json = serde_json::to_string(&wake).unwrap();
    let back: Wake = serde_json::from_str(&json).unwrap();
    assert_eq!(wake, back);

    let created = WakeRecord::Wake { wake: wake.clone() };
    let json = serde_json::to_string(&created).unwrap();
    let back: WakeRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(created, back);

    let delivered = WakeRecord::Delivered {
        id: wake.id.clone(),
        at: Timestamp::from_secs(1_700_000_100),
    };
    let json = serde_json::to_string(&delivered).unwrap();
    let back: WakeRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(delivered, back);
}

#[test]
fn registry_round_trips_through_toml() {
    let mut repos = BTreeMap::new();
    repos.insert(
        RepoId::from("bulkhead"),
        RegistryEntry {
            path: PathBuf::from("/Users/captain/bulkhead"),
            origin: Some("git@github.com:captain/bulkhead.git".to_owned()),
            workspace: Some(WorkspaceId::from("w1")),
            delivery: DeliveryMode::NoMistakes,
        },
    );
    let registry = Registry { repos };

    let text = toml::to_string(&registry).unwrap();
    let back: Registry = toml::from_str(&text).unwrap();
    assert_eq!(registry, back);
}

#[test]
fn repo_config_uses_documented_defaults() {
    let config: RepoConfig = toml::from_str("").unwrap();

    assert_eq!(config.delivery, DeliveryMode::NoMistakes);
    assert_eq!(config.max_crew, 3);
    assert_eq!(config.stall_after, Seconds::from_secs(20 * 60));
    assert_eq!(config.rebase_after, Seconds::from_secs(60 * 60));
    assert!(config.away.push);
    assert!(config.skills.is_empty());
    assert!(!config.yolo);
}

#[test]
fn repo_config_round_trips_through_toml() {
    let config = RepoConfig {
        delivery: DeliveryMode::DirectPr,
        max_crew: 5,
        stall_after: Seconds::from_secs(600),
        rebase_after: Seconds::from_secs(1800),
        away: bh_core::AwayPolicy { push: false },
        skills: vec!["spec-units".to_owned(), "langgraph".to_owned()],
        yolo: true,
    };

    let text = toml::to_string(&config).unwrap();
    let back: RepoConfig = toml::from_str(&text).unwrap();
    assert_eq!(config, back);
}

#[test]
fn task_round_trips_through_toml() {
    let task = Task {
        id: TaskId::from("t-1"),
        repo: RepoId::from("bulkhead"),
        title: "Add classifier".to_owned(),
        worktree: PathBuf::from("/tmp/treehouse/t-1"),
        tab: TabId::from("w1:t2"),
        pane: PaneId::from("w1:p3"),
        session: SessionRef::from("/Users/captain/.pi/sessions/t-1.jsonl"),
        pr_url: Some("https://github.com/captain/bulkhead/pull/1".to_owned()),
        created_at: Timestamp::from_secs(1_700_000_000),
        updated_at: Timestamp::from_secs(1_700_000_001),
    };

    let text = toml::to_string(&task).unwrap();
    let back: Task = toml::from_str(&text).unwrap();
    assert_eq!(task, back);
}

#[test]
fn protocol_envelope_round_trips_and_rejects_wrong_major() {
    let envelope = Envelope::new(ProtocolMessage::Hello {
        repo: RepoId::from("bulkhead"),
        pid: 42,
    });

    let line = envelope.encode().unwrap();
    assert!(line.ends_with('\n'));
    let decoded = Envelope::decode(&line).unwrap();
    assert_eq!(envelope, decoded);

    let wrong_major = PROTOCOL_MAJOR + 1;
    let bad = format!(
        "{{\"version\":{{\"major\":{wrong_major},\"minor\":0}},\
         \"message\":{{\"type\":\"error\",\"message\":\"nope\"}}}}"
    );
    match Envelope::decode(&bad) {
        Err(ProtocolError::UnsupportedVersion { found, ours }) => {
            assert_eq!(found, wrong_major);
            assert_eq!(ours, PROTOCOL_MAJOR);
        }
        other => panic!("expected version rejection, got {other:?}"),
    }
}

#[test]
fn protocol_ack_round_trips() {
    let envelope = Envelope::new(ProtocolMessage::Ack {
        wake: WakeId::from("w-1"),
    });
    let line = envelope.encode().unwrap();
    let decoded = Envelope::decode(&line).unwrap();
    assert_eq!(envelope, decoded);
}
