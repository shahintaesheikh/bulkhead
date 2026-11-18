//! `bh ls` summary tests.

use std::collections::BTreeMap;
use std::fs;

use bh::herdr::WorkspaceInfo;
use bh::ls::summarize;
use bh::paths::BulkheadPaths;
use bh_core::{DeliveryMode, Registry, RegistryEntry, RepoId, WorkspaceId};

#[test]
fn summarizes_repos_tasks_and_workspace_labels() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = BulkheadPaths {
        home: tmp.path().join("bulkhead"),
        global_agent: tmp.path().join("pi-agent"),
    };
    let repo = RepoId::from("bulkhead");
    fs::create_dir_all(paths.repo_home(&repo).join("tasks/t-1")).unwrap();
    fs::create_dir_all(paths.repo_home(&repo).join("tasks/t-2")).unwrap();

    let mut repos = BTreeMap::new();
    repos.insert(
        repo.clone(),
        RegistryEntry {
            path: tmp.path().join("checkout"),
            origin: None,
            workspace: Some(WorkspaceId::from("w1")),
            delivery: DeliveryMode::NoMistakes,
        },
    );
    let registry = Registry { repos };
    let workspaces = vec![WorkspaceInfo {
        workspace_id: WorkspaceId::from("w1"),
        label: "bulkhead".to_owned(),
        active_tab_id: None,
    }];

    let rows = summarize(&paths, &registry, &workspaces).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, repo);
    assert_eq!(rows[0].tasks, 2);
    assert_eq!(rows[0].workspace_label.as_deref(), Some("bulkhead"));
}
