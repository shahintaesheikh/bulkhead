//! Building a repo's `PI_CODING_AGENT_DIR`.
//!
//! The folder inherits the global layer by symlinking the global files that
//! exist, then generates the files bulkhead owns. Pi resolves its entire global
//! layer from this folder, so anything not linked or generated is absent.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use anyhow::{Context, Result, bail};
use bh_core::RepoId;
use serde_json::{Value, json};

use crate::fsx::write_if_missing;
use crate::paths::BulkheadPaths;

/// Global files linked into every repo's agent folder, when they exist.
pub const LINKED: &[&str] = &["AGENTS.md", "auth.json", "models.json", "themes"];

/// What happened to one link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOutcome {
    /// A new symlink was created or an existing one refreshed.
    Linked,
    /// The correct symlink was already in place.
    AlreadyLinked,
    /// The global source does not exist, so nothing was linked.
    SourceMissing,
}

/// Builds the agent folder and returns each link's outcome.
pub fn build(paths: &BulkheadPaths, repo: &RepoId) -> Result<Vec<(&'static str, LinkOutcome)>> {
    let agent = paths.agent_dir(repo);
    fs::create_dir_all(&agent).with_context(|| format!("creating {}", agent.display()))?;
    for dir in ["skills", "extensions", "prompts"] {
        let path = agent.join(dir);
        fs::create_dir_all(&path).with_context(|| format!("creating {}", path.display()))?;
    }

    let mut outcomes = Vec::with_capacity(LINKED.len());
    for name in LINKED {
        outcomes.push((*name, link_global(&paths.global_agent, &agent, name)?));
    }

    link_shared_assets(paths, repo, &agent)?;

    write_if_missing(
        &agent.join("settings.json"),
        &render_settings(&paths.global_agent)?,
    )?;
    write_if_missing(&agent.join("mcp.json"), &render_mcp()?)?;

    Ok(outcomes)
}

/// Links opted-in shared skills and every shared prompt template.
///
/// A skill is opt-in through the repo's `skills` list; a prompt template is
/// linked whenever it is present in `shared/prompts/`, since an unused slash
/// command costs no context.
fn link_shared_assets(paths: &BulkheadPaths, repo: &RepoId, agent: &Path) -> Result<()> {
    let config = crate::config::load(paths, repo)?;
    for name in &config.skills {
        link_path(
            &paths.shared_dir().join("skills").join(name),
            &agent.join("skills").join(name),
        )?;
    }

    let prompts = paths.shared_dir().join("prompts");
    if prompts.is_dir() {
        for entry in
            fs::read_dir(&prompts).with_context(|| format!("reading {}", prompts.display()))?
        {
            let source = entry?.path();
            if let Some(name) = source.file_name() {
                link_path(&source, &agent.join("prompts").join(name))?;
            }
        }
    }

    Ok(())
}

/// Symlinks `global_agent/<name>` into `agent/<name>`.
fn link_global(global_agent: &Path, agent: &Path, name: &str) -> Result<LinkOutcome> {
    link_path(&global_agent.join(name), &agent.join(name))
}

fn link_path(source: &Path, dest: &Path) -> Result<LinkOutcome> {
    if !source.exists() {
        return Ok(LinkOutcome::SourceMissing);
    }

    match fs::symlink_metadata(dest) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if matches!(fs::read_link(dest), Ok(current) if current.as_path() == source) {
                return Ok(LinkOutcome::AlreadyLinked);
            }
            fs::remove_file(dest)
                .with_context(|| format!("replacing stale link {}", dest.display()))?;
        }
        Ok(_) => bail!(
            "refusing to overwrite {}: it exists and is not a symlink",
            dest.display()
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(err).with_context(|| format!("inspecting {}", dest.display()));
        }
    }

    symlink(source, dest)
        .with_context(|| format!("linking {} -> {}", dest.display(), source.display()))?;
    Ok(LinkOutcome::Linked)
}

/// Seeds `settings.json` with the global default provider and model.
///
/// Rationale: with `PI_CODING_AGENT_DIR` set, Pi reads settings only from this
/// folder, so an empty file would leave an unattended agent without a model.
fn render_settings(global_agent: &Path) -> Result<String> {
    let mut settings = serde_json::Map::new();

    let global = global_agent.join("settings.json");
    if let Ok(text) = fs::read_to_string(&global) {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            for key in ["defaultProvider", "defaultModel"] {
                if let Some(found) = value.get(key) {
                    settings.insert(key.to_owned(), found.clone());
                }
            }
        }
    }

    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Object(settings))?
    ))
}

fn render_mcp() -> Result<String> {
    let value = json!({ "mcpServers": {} });
    Ok(format!("{}\n", serde_json::to_string_pretty(&value)?))
}
