//! `bh attach` integration tests against the scripted fake herdr.
//!
//! No test touches a real herdr server or the real home directories.

use std::fs;
use std::path::{Path, PathBuf};

use bh::herdr::Herdr;
use bh::paths::BulkheadPaths;
use bh::repo::DetectedRepo;
use bh::{attach, init, registry};
use bh_core::RepoId;

fn test_paths(tmp: &Path) -> BulkheadPaths {
    BulkheadPaths {
        home: tmp.join("bulkhead"),
        global_agent: tmp.join("pi-agent"),
    }
}

fn seed_global_agent(agent: &Path) {
    fs::create_dir_all(agent.join("themes")).unwrap();
    fs::write(agent.join("auth.json"), "{}").unwrap();
    fs::write(agent.join("AGENTS.md"), "# global").unwrap();
    fs::write(agent.join("settings.json"), "{}").unwrap();
}

fn repo_fixture(tmp: &Path) -> DetectedRepo {
    let root = tmp.join("bulkhead");
    fs::create_dir_all(&root).unwrap();
    DetectedRepo {
        root,
        origin: Some("git@github.com:shahintaesheikh/bulkhead.git".to_owned()),
        id: RepoId::from("bulkhead"),
    }
}

fn fake_herdr() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/fake-herdr")
}

fn adapter(state: &Path) -> Herdr {
    Herdr::new(fake_herdr(), "bulkhead")
        .with_child_env("FAKE_HERDR_STATE", state.to_string_lossy().into_owned())
}

#[test]
fn attach_ensure_builds_layout_and_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = test_paths(tmp.path());
    seed_global_agent(&paths.global_agent);
    let detected = repo_fixture(tmp.path());
    init::init(&paths, &detected).unwrap();

    let state = tmp.path().join("herdr-state");
    let herdr = adapter(&state);

    let plan = attach::ensure(&paths, &herdr, &detected).unwrap();
    assert!(plan.started_server);
    assert!(plan.created_workspace);
    assert!(plan.created_tab);
    assert!(plan.started_orchestrator);
    assert_eq!(plan.workspace.as_str(), "w1");
    assert_eq!(plan.tab.as_str(), "w1:t2");
    assert_eq!(plan.pane.as_str(), "w1:p2");

    let registry = registry::load(&paths).unwrap();
    assert_eq!(
        registry.repos[&detected.id]
            .workspace
            .as_ref()
            .unwrap()
            .as_str(),
        "w1"
    );

    // A second run reuses the server, workspace, tab and orchestrator.
    let plan = attach::ensure(&paths, &herdr, &detected).unwrap();
    assert!(!plan.started_server);
    assert!(!plan.created_workspace);
    assert!(!plan.created_tab);
    assert!(!plan.started_orchestrator);
    assert_eq!(plan.pane.as_str(), "w1:p2");

    let calls = fs::read_to_string(state.join("calls.log")).unwrap();
    assert!(
        calls.contains("--session bulkhead status server"),
        "{calls}"
    );
    assert!(calls.contains("--session bulkhead server"), "{calls}");
    assert!(
        calls.contains("--session bulkhead workspace create"),
        "{calls}"
    );
    assert!(calls.contains("--env PI_CODING_AGENT_DIR="), "{calls}");
    assert!(
        calls.contains("--session bulkhead agent start orch --kind pi --pane w1:p2"),
        "{calls}"
    );
}

#[test]
fn attach_adopts_a_workspace_matching_the_repo_label() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = test_paths(tmp.path());
    seed_global_agent(&paths.global_agent);
    let detected = repo_fixture(tmp.path());
    init::init(&paths, &detected).unwrap();

    let state = tmp.path().join("herdr-state");
    let herdr = adapter(&state);

    // A workspace already exists out of band, labeled with the repo id.
    herdr.start_server().unwrap();
    let created = herdr
        .create_workspace(&detected.root, detected.id.as_str(), &[])
        .unwrap();
    assert_eq!(created.workspace.workspace_id.as_str(), "w1");

    let plan = attach::ensure(&paths, &herdr, &detected).unwrap();
    assert!(!plan.started_server);
    assert!(!plan.created_workspace);
    assert_eq!(plan.workspace.as_str(), "w1");
    assert!(plan.created_tab);
    assert!(plan.started_orchestrator);

    // Adoption is persisted even though nothing was created.
    let registry = registry::load(&paths).unwrap();
    assert_eq!(
        registry.repos[&detected.id]
            .workspace
            .as_ref()
            .unwrap()
            .as_str(),
        "w1"
    );
}

#[test]
fn attach_recreates_a_workspace_that_disappeared() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = test_paths(tmp.path());
    seed_global_agent(&paths.global_agent);
    let detected = repo_fixture(tmp.path());
    init::init(&paths, &detected).unwrap();

    let state = tmp.path().join("herdr-state");
    let herdr = adapter(&state);
    let first = attach::ensure(&paths, &herdr, &detected).unwrap();
    assert_eq!(first.workspace.as_str(), "w1");

    // The server came back without the recorded workspace but with another.
    fs::write(state.join("server"), "restarted").unwrap();
    fs::write(
        state.join("workspaces.json"),
        r#"[{"workspace_id":"w9","label":"other","active_tab_id":"w9:t1"}]"#,
    )
    .unwrap();

    let plan = attach::ensure(&paths, &herdr, &detected).unwrap();
    assert!(plan.created_workspace);
    assert_eq!(plan.workspace.as_str(), "w2");

    let registry = registry::load(&paths).unwrap();
    assert_eq!(
        registry.repos[&detected.id]
            .workspace
            .as_ref()
            .unwrap()
            .as_str(),
        "w2"
    );
}
