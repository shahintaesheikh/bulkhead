//! Away-mode and return-brief tests.

use std::fs;

use bh::paths::BulkheadPaths;
use bh::repo::DetectedRepo;
use bh::{away, init};
use bh_core::RepoId;

#[test]
fn toggling_away_controls_the_file_and_brief() {
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

    let state = paths.repo_home(&repo.id).join("state");
    fs::write(
        state.join("digest.md"),
        "- 1700000000 t-1 blocked: needs a key\n",
    )
    .unwrap();
    let tasks = paths.repo_home(&repo.id).join("tasks/t-1");
    fs::create_dir_all(&tasks).unwrap();

    away::set(&paths, &repo.id, true).unwrap();
    assert!(state.join("away.json").is_file());

    away::set(&paths, &repo.id, false).unwrap();
    assert!(!state.join("away.json").exists());

    let brief = away::brief(&paths, &repo.id).unwrap();
    assert!(brief.contains("needs a key"), "{brief}");
    assert!(brief.contains("t-1"), "{brief}");
}
