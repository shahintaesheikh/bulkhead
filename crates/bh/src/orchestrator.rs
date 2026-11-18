//! The orchestrator contract and extension, embedded and installed per repo.
//!
//! Both live in the bulkhead source tree and are embedded with `include_str!`,
//! so the copy written into a repo always matches the running binary. Pi loads
//! the extension from the agent folder by discovery; the contract is injected
//! on the orchestrator's command line with `--append-system-prompt`.

use std::path::{Path, PathBuf};

use anyhow::Result;
use bh_core::RepoId;

use crate::fsx::write_atomic;
use crate::paths::BulkheadPaths;

/// The v1 orchestrator contract.
pub const CONTRACT: &str = include_str!("../../../prompts/orchestrator-contract.md");

/// The `bulkhead-orch` extension entry point.
pub const EXTENSION: &str = include_str!("../../../extensions/bulkhead-orch/src/extension.ts");

/// Where the contract is written for a repo.
pub fn contract_path(paths: &BulkheadPaths, repo: &RepoId) -> PathBuf {
    paths
        .agent_dir(repo)
        .join("prompts/orchestrator-contract.md")
}

/// Where the extension entry point is written for a repo.
pub fn extension_entry(paths: &BulkheadPaths, repo: &RepoId) -> PathBuf {
    paths
        .agent_dir(repo)
        .join("extensions/bulkhead-orch/index.ts")
}

/// Writes the contract and extension into a repo's agent folder.
///
/// Both are bulkhead-owned and versioned with the code, so they are always
/// overwritten; `bh init` and `bh attach` both call this.
pub fn install(paths: &BulkheadPaths, repo: &RepoId) -> Result<()> {
    write_atomic(&contract_path(paths, repo), CONTRACT)?;
    write_atomic(&extension_entry(paths, repo), EXTENSION)?;
    Ok(())
}

/// Pi arguments that append the contract to the orchestrator's system prompt.
pub fn launch_args(contract: &Path) -> Vec<String> {
    vec![
        "--append-system-prompt".to_owned(),
        contract.to_string_lossy().into_owned(),
    ]
}
