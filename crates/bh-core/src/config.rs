//! Registry and per-repo configuration.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::{RepoId, WorkspaceId};
use crate::time::Seconds;

/// `~/.bulkhead/registry.toml`: repo id to registration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Registry {
    /// Registered repos, keyed by id.
    #[serde(default)]
    pub repos: BTreeMap<RepoId, RegistryEntry>,
}

/// One registered repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Absolute path to the primary checkout.
    pub path: PathBuf,
    /// Origin remote URL, if the repo has one.
    #[serde(default)]
    pub origin: Option<String>,
    /// herdr workspace bound to this repo.
    pub workspace: WorkspaceId,
    /// How finished work reaches main.
    #[serde(default)]
    pub delivery: DeliveryMode,
}

/// How finished work reaches main.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryMode {
    /// Route through the no-mistakes PR gate.
    #[default]
    NoMistakes,
    /// Open a PR directly with gh-axi.
    DirectPr,
    /// Keep the branch local; the captain merges by hand.
    Local,
}

/// `~/.bulkhead/<repo-id>/config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoConfig {
    /// How finished work reaches main.
    #[serde(default)]
    pub delivery: DeliveryMode,
    /// Maximum crewmates running at once.
    #[serde(default = "default_max_crew")]
    pub max_crew: u32,
    /// How long a silent crewmate may run before it is considered stalled.
    #[serde(default = "default_stall_after")]
    pub stall_after: Seconds,
    /// How long a PR may wait before `bhd` suggests rebasing it.
    #[serde(default = "default_rebase_after")]
    pub rebase_after: Seconds,
    /// Away-mode behavior.
    #[serde(default)]
    pub away: AwayPolicy,
    /// Names of opt-in skills from `~/.bulkhead/shared/`.
    #[serde(default)]
    pub skills: Vec<String>,
    /// Allow merge on green for this repo without asking the captain.
    #[serde(default)]
    pub yolo: bool,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            delivery: DeliveryMode::default(),
            max_crew: default_max_crew(),
            stall_after: default_stall_after(),
            rebase_after: default_rebase_after(),
            away: AwayPolicy::default(),
            skills: Vec::new(),
            yolo: false,
        }
    }
}

/// Away-mode policy for one repo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwayPolicy {
    /// Send a phone push when the orchestrator needs the captain.
    #[serde(default = "default_true")]
    pub push: bool,
}

impl Default for AwayPolicy {
    fn default() -> Self {
        Self { push: true }
    }
}

fn default_true() -> bool {
    true
}

fn default_max_crew() -> u32 {
    3
}

fn default_stall_after() -> Seconds {
    Seconds::from_secs(20 * 60)
}

fn default_rebase_after() -> Seconds {
    Seconds::from_secs(60 * 60)
}
