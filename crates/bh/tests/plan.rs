//! Plan intake and task backlog tests.

use std::fs;
use std::path::{Path, PathBuf};

use bh::lavish::LavishAxi;
use bh::paths::BulkheadPaths;
use bh::repo::DetectedRepo;
use bh::{backlog, init, plan};
use bh_core::{RepoId, TriageState};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn setup() -> (tempfile::TempDir, BulkheadPaths, DetectedRepo) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = BulkheadPaths {
        home: tmp.path().join("bulkhead"),
        global_agent: tmp.path().join("pi-agent"),
    };
    fs::create_dir_all(paths.global_agent.join("themes")).unwrap();
    fs::write(paths.global_agent.join("auth.json"), "{}").unwrap();
    fs::write(paths.global_agent.join("AGENTS.md"), "# g").unwrap();
    fs::write(paths.global_agent.join("settings.json"), "{}").unwrap();
    let root = tmp.path().join("bulkhead");
    fs::create_dir_all(&root).unwrap();
    let repo = DetectedRepo {
        root,
        origin: None,
        id: RepoId::from("bulkhead"),
    };
    init::init(&paths, &repo).unwrap();
    (tmp, paths, repo)
}

#[test]
fn task_creates_a_backlog_item_and_dedupes_ids() {
    let (_tmp, paths, repo) = setup();

    let id = backlog::create(&paths, &repo.id, "Add a parser\n\nWith tests.").unwrap();
    assert_eq!(id.as_str(), "add-a-parser");

    let file = paths.repo_home(&repo.id).join("backlog/add-a-parser.md");
    let text = fs::read_to_string(&file).unwrap();
    assert!(text.contains("title: Add a parser"), "{text}");
    assert!(text.contains("state: backlog"), "{text}");

    let items = backlog::list(&paths, &repo.id).unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "Add a parser");
    assert_eq!(items[0].state, "backlog");

    let second = backlog::create(&paths, &repo.id, "Add a parser").unwrap();
    assert_eq!(second.as_str(), "add-a-parser-2");
}

#[test]
fn plans_ingest_and_triage_between_buckets() {
    let (tmp, paths, repo) = setup();

    let source = tmp.path().join("spec.md");
    fs::write(&source, "# Socket format\n\nUse JSON lines.\n").unwrap();
    let plan = plan::ingest(&paths, &repo.id, &source, None).unwrap();
    assert_eq!(plan.id.as_str(), "spec");
    assert_eq!(plan.state, TriageState::Inbox);
    assert_eq!(plan.title, "Socket format");
    assert!(
        paths
            .repo_home(&repo.id)
            .join("plans/inbox/spec.md")
            .is_file()
    );

    let accepted =
        plan::set_state(&paths, &repo.id, &plan.id, TriageState::Accepted, None).unwrap();
    assert_eq!(accepted.state, TriageState::Accepted);
    assert!(
        paths
            .repo_home(&repo.id)
            .join("plans/accepted/spec.md")
            .is_file()
    );
    assert!(
        !paths
            .repo_home(&repo.id)
            .join("plans/inbox/spec.md")
            .exists()
    );

    let listed = plan::list(&paths, &repo.id).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].state, TriageState::Accepted);

    let source2 = tmp.path().join("other.md");
    fs::write(&source2, "# Other\n").unwrap();
    let second = plan::ingest(&paths, &repo.id, &source2, None).unwrap();
    let declined = plan::set_state(
        &paths,
        &repo.id,
        &second.id,
        TriageState::Declined,
        Some("out of scope".to_owned()),
    )
    .unwrap();
    assert_eq!(declined.reason.as_deref(), Some("out of scope"));
    assert!(
        paths
            .repo_home(&repo.id)
            .join("plans/declined/other.md")
            .is_file()
    );
}

#[test]
fn html_plans_get_a_lavish_review_url() {
    let (tmp, paths, repo) = setup();
    let lavish_state = tmp.path().join("lavish-state");
    let lavish = LavishAxi::new(fixture("fake-lavish").to_string_lossy().into_owned(), &[])
        .with_child_env(
            "FAKE_LAVISH_STATE",
            lavish_state.to_string_lossy().into_owned(),
        );

    let source = tmp.path().join("design.html");
    fs::write(&source, "<html><body>Design</body></html>").unwrap();
    let plan = plan::ingest(&paths, &repo.id, &source, Some(&lavish)).unwrap();

    assert_eq!(
        plan.review_url.as_deref(),
        Some("https://lavish.example/review/1")
    );
    let calls = fs::read_to_string(lavish_state.join("calls.log")).unwrap();
    assert!(calls.contains("publish "), "{calls}");
}
