//! Opt-in shared skill and shared prompt linking.

use std::fs;

use bh::agent_folder;
use bh::paths::BulkheadPaths;
use bh::repo::DetectedRepo;
use bh_core::RepoId;

#[test]
fn links_opted_in_skills_and_shared_prompts() {
    let tmp = tempfile::tempdir().unwrap();
    let paths = BulkheadPaths {
        home: tmp.path().join("bulkhead"),
        global_agent: tmp.path().join("pi-agent"),
    };
    fs::create_dir_all(&paths.global_agent).unwrap();
    fs::write(paths.global_agent.join("auth.json"), "{}").unwrap();

    let repo = DetectedRepo {
        root: tmp.path().join("bulkhead"),
        origin: None,
        id: RepoId::from("bulkhead"),
    };

    // Shared assets.
    let skill = paths.shared_dir().join("skills/spec-units");
    fs::create_dir_all(&skill).unwrap();
    fs::write(skill.join("SKILL.md"), "# spec-units\n").unwrap();
    fs::create_dir_all(paths.shared_dir().join("prompts")).unwrap();
    fs::write(
        paths.shared_dir().join("prompts/bootstrap-context.md"),
        "# /bootstrap-context\n",
    )
    .unwrap();

    // The repo opts into the skill.
    let home = paths.repo_home(&repo.id);
    fs::create_dir_all(home.join("agent")).unwrap();
    fs::write(home.join("config.toml"), "skills = [\"spec-units\"]\n").unwrap();

    agent_folder::build(&paths, &repo.id).unwrap();

    let agent = paths.agent_dir(&repo.id);
    assert!(
        fs::symlink_metadata(agent.join("skills/spec-units"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::symlink_metadata(agent.join("prompts/bootstrap-context.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
