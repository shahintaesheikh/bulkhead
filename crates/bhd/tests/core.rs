//! Core loop integration tests: status and plan events become durable wakes.

use std::fs;
use std::path::PathBuf;

use bh_core::protocol::{Envelope, ProtocolMessage};
use bh_core::{RepoId, WakeKind};
use bhd::core::Core;
use bhd::paths::Paths;
use bhd::source::SourceEvent;
use tokio::sync::mpsc;

fn setup() -> (tempfile::TempDir, Paths, RepoId, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths {
        home: tmp.path().join("bulkhead"),
        global_agent: tmp.path().join("pi-agent"),
    };
    let repo = RepoId::from("bulkhead");
    let task_dir = paths.repo_home(&repo).join("tasks/t-1");
    fs::create_dir_all(&task_dir).unwrap();
    let log = task_dir.join("status.log");
    fs::write(&log, "").unwrap();
    (tmp, paths, repo, log)
}

#[tokio::test]
async fn status_line_becomes_a_durable_delivered_wake() {
    let (_tmp, paths, repo, log) = setup();
    let mut core = Core::new(paths.clone(), std::iter::once(repo.clone())).unwrap();

    let (client, mut received) = mpsc::unbounded_channel::<Envelope>();
    core.handle(SourceEvent::Connected {
        repo: repo.clone(),
        sender: client,
    })
    .unwrap();

    fs::write(&log, "blocked: needs a key\n").unwrap();
    core.handle(SourceEvent::Filesystem(vec![log.clone()]))
        .unwrap();
    core.handle(SourceEvent::Tick).unwrap();

    let envelope = received.try_recv().expect("a wake was delivered");
    let wake = match envelope.message {
        ProtocolMessage::Wake { wake } => wake,
        other => panic!("expected a wake, got {other:?}"),
    };
    assert_eq!(wake.kind, WakeKind::Blocked);
    assert_eq!(wake.task.as_ref().map(|task| task.as_str()), Some("t-1"));

    let raw = fs::read_to_string(paths.wakes_file(&repo)).unwrap();
    assert!(raw.contains("\"record\":\"wake\""), "{raw}");

    core.handle(SourceEvent::Message {
        repo: repo.clone(),
        message: Envelope::new(ProtocolMessage::Ack {
            wake: wake.id.clone(),
        }),
    })
    .unwrap();

    let raw = fs::read_to_string(paths.wakes_file(&repo)).unwrap();
    assert!(raw.contains("\"record\":\"delivered\""), "{raw}");
    assert_eq!(core.pending(&repo), 0);
}

#[tokio::test]
async fn plan_arrival_wakes_only_once() {
    let (_tmp, paths, repo, _log) = setup();
    let mut core = Core::new(paths.clone(), std::iter::once(repo.clone())).unwrap();
    let (client, mut received) = mpsc::unbounded_channel::<Envelope>();
    core.handle(SourceEvent::Connected {
        repo: repo.clone(),
        sender: client,
    })
    .unwrap();

    let inbox = paths.repo_home(&repo).join("plans/inbox");
    fs::create_dir_all(&inbox).unwrap();
    let plan = inbox.join("spec.md");
    fs::write(&plan, "# Spec\n").unwrap();

    core.handle(SourceEvent::Filesystem(vec![plan.clone()]))
        .unwrap();
    core.handle(SourceEvent::Tick).unwrap();
    let first = match received.try_recv().unwrap().message {
        ProtocolMessage::Wake { wake } => wake,
        other => panic!("expected a wake, got {other:?}"),
    };

    // The same plan must not create a second wake; a redelivery keeps its id.
    core.handle(SourceEvent::Filesystem(vec![plan])).unwrap();
    core.handle(SourceEvent::Tick).unwrap();
    let again = match received.try_recv().unwrap().message {
        ProtocolMessage::Wake { wake } => wake,
        other => panic!("expected a wake, got {other:?}"),
    };
    assert_eq!(first.id, again.id);

    let raw = fs::read_to_string(paths.wakes_file(&repo)).unwrap();
    assert_eq!(raw.matches("\"record\":\"wake\"").count(), 1, "{raw}");
}
