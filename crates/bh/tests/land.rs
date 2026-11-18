//! Landing tests across delivery modes, against the scripted fakes.

use std::fs;
use std::path::{Path, PathBuf};

use bh::delivery::{self, Landers};
use bh::ghaxi::GhAxi;
use bh::herdr::Herdr;
use bh::no_mistakes::NoMistakes;
use bh::paths::BulkheadPaths;
use bh::repo::DetectedRepo;
use bh::treehouse::Treehouse;
use bh::{attach, init, task};
use bh_core::{DeliveryMode, RepoId, TaskId};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

struct Env {
    _tmp: tempfile::TempDir,
    paths: BulkheadPaths,
    repo: DetectedRepo,
    herdr: Herdr,
    treehouse: Treehouse,
    ghaxi: GhAxi,
    no_mistakes: NoMistakes,
    herdr_state: PathBuf,
    th_state: PathBuf,
    ghaxi_state: PathBuf,
    nm_state: PathBuf,
}

impl Env {
    fn landers(&self) -> Landers<'_> {
        Landers {
            herdr: &self.herdr,
            treehouse: &self.treehouse,
            ghaxi: &self.ghaxi,
            no_mistakes: &self.no_mistakes,
        }
    }
}

fn setup() -> Env {
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

    let herdr_state = tmp.path().join("herdr-state");
    let herdr = Herdr::new(fixture("fake-herdr"), "bulkhead").with_child_env(
        "FAKE_HERDR_STATE",
        herdr_state.to_string_lossy().into_owned(),
    );
    attach::ensure(&paths, &herdr, &repo).unwrap();

    let th_state = tmp.path().join("th-state");
    let treehouse = Treehouse::new(fixture("fake-treehouse")).with_child_env(
        "FAKE_TREEHOUSE_STATE",
        th_state.to_string_lossy().into_owned(),
    );
    let ghaxi_state = tmp.path().join("ghaxi-state");
    let ghaxi = GhAxi::new(fixture("fake-ghaxi").to_string_lossy().into_owned(), &[])
        .with_child_env(
            "FAKE_GHAXI_STATE",
            ghaxi_state.to_string_lossy().into_owned(),
        );
    let nm_state = tmp.path().join("nm-state");
    let no_mistakes = NoMistakes::new(fixture("fake-no-mistakes")).with_child_env(
        "FAKE_NOMISTAKES_STATE",
        nm_state.to_string_lossy().into_owned(),
    );

    Env {
        _tmp: tmp,
        paths,
        repo,
        herdr,
        treehouse,
        ghaxi,
        no_mistakes,
        herdr_state,
        th_state,
        ghaxi_state,
        nm_state,
    }
}

fn spawn_task(env: &Env, id: &str) {
    let backlog = env.paths.repo_home(&env.repo.id).join("backlog");
    fs::create_dir_all(&backlog).unwrap();
    fs::write(
        backlog.join(format!("{id}.md")),
        format!("# {id}\n\nbody\n"),
    )
    .unwrap();
    task::spawn(
        &env.paths,
        &env.herdr,
        &env.treehouse,
        &env.repo,
        &TaskId::from(id),
    )
    .unwrap();
}

fn set_delivery(env: &Env, mode: &str) {
    let config = env.paths.repo_home(&env.repo.id).join("config.toml");
    let text = fs::read_to_string(&config).unwrap();
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| !line.starts_with("delivery"))
        .collect();
    fs::write(
        &config,
        format!("delivery = \"{mode}\"\n{}\n", kept.join("\n")),
    )
    .unwrap();
}

fn set_task_pr(env: &Env, id: &str, pr: &str) {
    let task_id = TaskId::from(id);
    let mut task = task::load_task(&env.paths, &env.repo.id, &task_id).unwrap();
    task.pr_url = Some(pr.to_owned());
    task::save_task(&env.paths, &env.repo.id, &task_id, &task).unwrap();
}

#[test]
fn land_requires_approval_then_merges_and_tears_down() {
    let env = setup();
    spawn_task(&env, "t-1");
    set_delivery(&env, "direct-pr");
    set_task_pr(&env, "t-1", "https://github.com/example/repo/pull/7");

    let err = delivery::land(
        &env.paths,
        &env.landers(),
        &env.repo.id,
        &TaskId::from("t-1"),
        false,
    )
    .unwrap_err();
    assert!(err.to_string().contains("approval"), "{err}");

    let report = delivery::land(
        &env.paths,
        &env.landers(),
        &env.repo.id,
        &TaskId::from("t-1"),
        true,
    )
    .unwrap();
    assert_eq!(report.mode, DeliveryMode::DirectPr);
    assert_eq!(report.reference, "7");

    let calls = fs::read_to_string(env.ghaxi_state.join("calls.log")).unwrap();
    assert!(
        calls.contains("pr merge 7 --method squash --delete-branch"),
        "{calls}"
    );

    let directory = task::task_dir(&env.paths, &env.repo.id, &TaskId::from("t-1"));
    assert!(directory.join("landed").is_file());
    let herdr_calls = fs::read_to_string(env.herdr_state.join("calls.log")).unwrap();
    assert!(herdr_calls.contains("tab close w1:t3"), "{herdr_calls}");
    let th_calls = fs::read_to_string(env.th_state.join("calls.log")).unwrap();
    assert!(th_calls.contains("release "), "{th_calls}");
}

#[test]
fn local_delivery_needs_no_approval_and_calls_no_gate() {
    let env = setup();
    spawn_task(&env, "t-1");
    set_delivery(&env, "local");

    let report = delivery::land(
        &env.paths,
        &env.landers(),
        &env.repo.id,
        &TaskId::from("t-1"),
        false,
    )
    .unwrap();
    assert_eq!(report.mode, DeliveryMode::Local);
    assert!(!env.ghaxi_state.join("calls.log").exists());
}

#[test]
fn no_mistakes_delivery_lands_through_the_gate() {
    let env = setup();
    spawn_task(&env, "t-1");
    set_delivery(&env, "no-mistakes");
    set_task_pr(&env, "t-1", "https://github.com/example/repo/pull/9");

    delivery::land(
        &env.paths,
        &env.landers(),
        &env.repo.id,
        &TaskId::from("t-1"),
        true,
    )
    .unwrap();
    let calls = fs::read_to_string(env.nm_state.join("calls.log")).unwrap();
    assert!(
        calls.contains("land https://github.com/example/repo/pull/9"),
        "{calls}"
    );
}

#[test]
fn land_lands_dependencies_first() {
    let env = setup();
    spawn_task(&env, "t-1");
    spawn_task(&env, "t-2");
    set_delivery(&env, "direct-pr");
    set_task_pr(&env, "t-1", "https://github.com/example/repo/pull/1");
    set_task_pr(&env, "t-2", "https://github.com/example/repo/pull/2");

    let task_id = TaskId::from("t-2");
    let mut task = task::load_task(&env.paths, &env.repo.id, &task_id).unwrap();
    task.depends_on = Some(TaskId::from("t-1"));
    task::save_task(&env.paths, &env.repo.id, &task_id, &task).unwrap();

    delivery::land(&env.paths, &env.landers(), &env.repo.id, &task_id, true).unwrap();

    let calls = fs::read_to_string(env.ghaxi_state.join("calls.log")).unwrap();
    let first = calls.find("pr merge 1").expect("dependency merged");
    let second = calls.find("pr merge 2").expect("task merged");
    assert!(first < second, "{calls}");
}
