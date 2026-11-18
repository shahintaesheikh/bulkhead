//! The `bh init` operation.

use std::path::PathBuf;

use anyhow::{Result, bail};
use bh_core::{DeliveryMode, RegistryEntry};

use crate::agent_folder::{self, LinkOutcome};
use crate::home;
use crate::paths::BulkheadPaths;
use crate::registry;
use crate::repo::DetectedRepo;

/// Summary of an `init` run.
#[derive(Debug)]
pub struct InitReport {
    /// Repo id that was registered.
    pub id: String,
    /// The repo's home directory.
    pub home: PathBuf,
    /// Outcome of each global-file link.
    pub links: Vec<(&'static str, LinkOutcome)>,
    /// Whether the repo was already in the registry.
    pub already_registered: bool,
}

/// Registers `repo` and builds its home and agent folder.
pub fn init(paths: &BulkheadPaths, repo: &DetectedRepo) -> Result<InitReport> {
    let mut registry = registry::load(paths)?;
    let already_registered = registry.repos.contains_key(&repo.id);

    if let Some(existing) = registry.repos.get(&repo.id) {
        if existing.path != repo.root {
            bail!(
                "repo id {} is already registered to {}",
                repo.id,
                existing.path.display()
            );
        }
    }

    home::create(paths, &repo.id)?;
    let links = agent_folder::build(paths, &repo.id)?;

    registry.repos.insert(
        repo.id.clone(),
        RegistryEntry {
            path: repo.root.clone(),
            origin: repo.origin.clone(),
            workspace: None,
            delivery: DeliveryMode::default(),
        },
    );
    registry::save(paths, &registry)?;

    Ok(InitReport {
        id: repo.id.to_string(),
        home: paths.repo_home(&repo.id),
        links,
        already_registered,
    })
}
