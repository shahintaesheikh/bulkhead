//! Crew lifecycle integration tests against the scripted fakes.

use std::fs;
use std::path::{Path, PathBuf};

use bh::herdr::Herdr;
use bh::paths::BulkheadPaths;
use bh::repo::DetectedRepo;
use bh::treehouse::Treehouse;
use bh::{attach, init, task};
use bh_core::TaskId;

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
        id: bh_core::RepoId::from("bulkhead"),
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

#[test]
fn spawn_send_status_teardown_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = test_paths(tmp.path());
    seed_global_agent(&paths.global_agent);
    let detected = repo_fixture(tmp.path());
    init::init(&paths, &detected).unwrap();

    let herdr_state = tmp.path().join("herdr-state");
    let herdr = Herdr::new(fixture("fake-herdr"), "bulkhead").with_child_env(
        "FAKE_HERDR_STATE",
        herdr_state.to_string_lossy().into_owned(),
    );
    let th_state = tmp.path().join("treehouse-state");
    let treehouse = Treehouse::new(fixture("fake-treehouse")).with_child_env(
        "FAKE_TREEHOUSE_STATE",
        th_state.to_string_lossy().into_owned(),
    );

    attach::ensure(&paths, &herdr, &detected).unwrap();

    let backlog = paths.repo_home(&detected.id).join("backlog");
    fs::write(
        backlog.join("t-1.md"),
        "# Add a parser\n\nBuild the parser and its tests.\n",
    )
    .unwrap();

    let task_id = TaskId::from("t-1");
    let report = task::spawn(&paths, &herdr, &treehouse, &detected, &task_id).unwrap();
    assert_eq!(report.tab.as_str(), "w1:t3");
    assert_eq!(report.pane.as_str(), "w1:p3");
    assert!(report.worktree.ends_with("worktrees/t-1"));

    let directory = task::task_dir(&paths, &detected.id, &task_id);
    let brief = fs::read_to_string(directory.join("brief.md")).unwrap();
    assert!(brief.contains("Add a parser"));
    assert!(brief.contains("Build the parser and its tests."));
    assert!(brief.contains(&report.worktree.display().to_string()));
    assert!(directory.join("status.log").is_file());

    let loaded = task::load_task(&paths, &detected.id, &task_id).unwrap();
    assert_eq!(loaded.pane.as_str(), "w1:p3");
    assert_eq!(loaded.title, "Add a parser");

    let calls = fs::read_to_string(herdr_state.join("calls.log")).unwrap();
    assert!(
        calls.contains(&format!(
            "tab create --no-focus --workspace w1 --cwd {}",
            report.worktree.display()
        )),
        "{calls}"
    );
    assert!(
        calls.contains("agent start t-1 --kind pi --pane w1:p3 -- --append-system-prompt"),
        "{calls}"
    );
    assert!(calls.contains("--session-id t-1"), "{calls}");

    task::send(&herdr, &paths, &detected.id, &task_id, "please add tests").unwrap();
    let prompts = fs::read_to_string(herdr_state.join("prompts.log")).unwrap();
    assert_eq!(prompts.trim(), "w1:p3\tplease add tests");

    fs::write(
        directory.join("status.log"),
        "working: building\nblocked: needs a key\n",
    )
    .unwrap();
    let status = task::status(&paths, &detected.id, Some(&task_id)).unwrap();
    assert!(status.contains("Add a parser"));
    assert!(status.contains("blocked: needs a key"));

    task::teardown(&herdr, &treehouse, &paths, &detected.id, &task_id).unwrap();
    let calls = fs::read_to_string(herdr_state.join("calls.log")).unwrap();
    assert!(calls.contains("tab close w1:t3"), "{calls}");
    let th_calls = fs::read_to_string(th_state.join("calls.log")).unwrap();
    assert!(
        th_calls.contains(&format!("release {}", report.worktree.display())),
        "{th_calls}"
    );
    assert!(directory.join("teardown").is_file());
}
