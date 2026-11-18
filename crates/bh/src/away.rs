//! Away-mode and the return brief.

use std::fs;

use anyhow::{Context, Result};
use bh_core::RepoId;

use crate::clock;
use crate::fsx;
use crate::paths::BulkheadPaths;

/// Turns away-mode on (writes `state/away.json`) or off (removes it).
pub fn set(paths: &BulkheadPaths, repo: &RepoId, away: bool) -> Result<()> {
    let file = paths.repo_home(repo).join("state/away.json");
    if away {
        let body = format!("{{\"started_at\":{}}}\n", clock::now().as_secs());
        fsx::write_atomic(&file, &body)
    } else if file.exists() {
        fs::remove_file(&file).with_context(|| format!("removing {}", file.display()))
    } else {
        Ok(())
    }
}

/// Builds the return brief from the digest and the task list.
pub fn brief(paths: &BulkheadPaths, repo: &RepoId) -> Result<String> {
    let home = paths.repo_home(repo);
    let mut output = String::from("# Return brief\n\n## Recent wakes\n");

    let digest = home.join("state/digest.md");
    if digest.exists() {
        let text =
            fs::read_to_string(&digest).with_context(|| format!("reading {}", digest.display()))?;
        let all: Vec<&str> = text.lines().collect();
        let lines: Vec<&str> = all.iter().rev().take(30).rev().copied().collect();
        if lines.is_empty() {
            output.push_str("(none)\n");
        }
        for line in lines {
            output.push_str(line);
            output.push('\n');
        }
    } else {
        output.push_str("(no wakes recorded)\n");
    }

    output.push_str("\n## Tasks\n");
    let tasks = home.join("tasks");
    let mut found = false;
    if tasks.is_dir() {
        for entry in fs::read_dir(&tasks).with_context(|| format!("reading {}", tasks.display()))? {
            let entry = entry?;
            if entry.path().is_dir() {
                found = true;
                output.push_str("- ");
                output.push_str(&entry.file_name().to_string_lossy());
                output.push('\n');
            }
        }
    }
    if !found {
        output.push_str("(none)\n");
    }

    Ok(output)
}
