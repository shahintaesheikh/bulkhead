//! End-to-end `init` and `doctor` tests against temporary homes.
//!
//! No test touches the real `~/.bulkhead`, `~/.pi` or `$HOME`.

use std::fs;
use std::path::Path;
use std::process::Command;

use bh::doctor::{self, Level};
use bh::paths::BulkheadPaths;
use bh::repo::{self, DetectedRepo};
use bh::{init, registry};
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
    fs::write(agent.join("models.json"), "{}").unwrap();
    fs::write(agent.join("AGENTS.md"), "# global instructions").unwrap();
    fs::write(
        agent.join("settings.json"),
        r#"{"defaultProvider":"openrouter","defaultModel":"deepseek/x"}"#,
    )
    .unwrap();
}

fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("git is installed");
    assert!(status.success(), "git {args:?} failed");
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

fn error_findings(findings: &[doctor::Finding]) -> Vec<String> {
    findings
        .iter()
        .filter(|finding| finding.level == Level::Error)
        .map(|finding| finding.message.clone())
        .collect()
}

#[test]
fn detect_reads_git_root_origin_and_slug() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_dir = tmp.path().join("Acme App");
    fs::create_dir_all(&repo_dir).unwrap();
    run_git(&repo_dir, &["init", "-q"]);
    run_git(
        &repo_dir,
        &["remote", "add", "origin", "git@github.com:acme/app.git"],
    );

    let detected = repo::detect(&repo_dir).unwrap();
    assert_eq!(detected.id.as_str(), "acme-app");
    assert_eq!(
        detected.origin.as_deref(),
        Some("git@github.com:acme/app.git")
    );
    assert_eq!(
        detected.root.canonicalize().unwrap(),
        repo_dir.canonicalize().unwrap()
    );
}

#[test]
fn init_builds_layout_and_links_and_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = test_paths(tmp.path());
    seed_global_agent(&paths.global_agent);
    let detected = repo_fixture(tmp.path());

    let report = init::init(&paths, &detected).unwrap();
    assert!(!report.already_registered);
    assert_eq!(report.id, "bulkhead");

    let home = paths.repo_home(&detected.id);
    for dir in bh::home::REPO_DIRS {
        assert!(home.join(dir).is_dir(), "{dir} is missing");
    }

    let agent = paths.agent_dir(&detected.id);
    for name in ["AGENTS.md", "auth.json", "models.json", "themes"] {
        let meta = fs::symlink_metadata(agent.join(name)).unwrap();
        assert!(meta.file_type().is_symlink(), "{name} should be a symlink");
    }
    assert!(agent.join("settings.json").is_file());
    assert!(agent.join("mcp.json").is_file());

    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(agent.join("settings.json")).unwrap()).unwrap();
    assert_eq!(settings["defaultModel"], "deepseek/x");

    let registry = registry::load(&paths).unwrap();
    let entry = &registry.repos[&detected.id];
    assert_eq!(entry.path, detected.root);
    assert_eq!(entry.workspace, None);

    // A second run must not fail or duplicate anything.
    let again = init::init(&paths, &detected).unwrap();
    assert!(again.already_registered);

    let findings = doctor::doctor(&paths).unwrap();
    assert!(
        error_findings(&findings).is_empty(),
        "doctor reported errors: {findings:#?}"
    );
}

#[test]
fn init_skips_absent_global_files() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = test_paths(tmp.path());
    fs::create_dir_all(&paths.global_agent).unwrap();
    fs::write(paths.global_agent.join("auth.json"), "{}").unwrap();
    let detected = repo_fixture(tmp.path());

    let report = init::init(&paths, &detected).unwrap();
    let outcomes: std::collections::BTreeMap<_, _> = report.links.into_iter().collect();
    assert_eq!(outcomes["auth.json"], bh::agent_folder::LinkOutcome::Linked);
    assert_eq!(
        outcomes["models.json"],
        bh::agent_folder::LinkOutcome::SourceMissing
    );
    assert!(!paths.agent_dir(&detected.id).join("models.json").exists());
}

#[test]
fn doctor_flags_broken_symlink_and_stray_agents_md() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = test_paths(tmp.path());
    seed_global_agent(&paths.global_agent);
    let detected = repo_fixture(tmp.path());
    init::init(&paths, &detected).unwrap();

    // Remove the global source so models.json becomes a dangling link.
    fs::remove_file(paths.global_agent.join("models.json")).unwrap();

    // Replace the AGENTS.md link with a real file.
    let agents_md = paths.agent_dir(&detected.id).join("AGENTS.md");
    fs::remove_file(&agents_md).unwrap();
    fs::write(&agents_md, "# stray copy").unwrap();

    let findings = doctor::doctor(&paths).unwrap();
    let errors = error_findings(&findings);
    assert!(
        errors
            .iter()
            .any(|message| message.contains("broken symlink")),
        "{findings:#?}"
    );
    assert!(
        errors
            .iter()
            .any(|message| message.contains("stray AGENTS.md")),
        "{findings:#?}"
    );
}

#[test]
fn init_refuses_a_repo_id_registered_elsewhere() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = test_paths(tmp.path());
    seed_global_agent(&paths.global_agent);

    let first = repo_fixture(tmp.path());
    init::init(&paths, &first).unwrap();

    let second = DetectedRepo {
        root: tmp.path().join("elsewhere").join("bulkhead"),
        origin: None,
        id: RepoId::from("bulkhead"),
    };
    let err = init::init(&paths, &second).unwrap_err();
    assert!(err.to_string().contains("already registered"), "{err}");
}
