//! The core loop: the only owner of mutable supervision state.
//!
//! Every event source hands [`SourceEvent`]s to [`Core::handle`], which reads
//! only what it owns, calls the pure `bh_core::classify`, and appends wakes to
//! their durable store before offering them to a client. No locks are shared
//! with a source (invariant 2).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bh_core::protocol::{Envelope, ProtocolMessage};
use bh_core::status::StatusLine;
use bh_core::{Event, PlanId, RepoId, SupervisionState, TaskId, Wake, WakeId};
use tokio::sync::mpsc;

use crate::clock;
use crate::paths::Paths;
use crate::source::SourceEvent;
use crate::wake_store::WakeStore;

/// The supervisor's state and its transitions.
pub struct Core {
    paths: Paths,
    state: SupervisionState,
    stores: BTreeMap<RepoId, WakeStore>,
    clients: BTreeMap<RepoId, mpsc::UnboundedSender<Envelope>>,
    offsets: BTreeMap<PathBuf, u64>,
    seen_plans: BTreeSet<(RepoId, String)>,
    away: BTreeMap<RepoId, bool>,
}

impl Core {
    /// Builds a core, rehydrating each repo's wake store.
    pub fn new(paths: Paths, repos: impl Iterator<Item = RepoId>) -> Result<Self> {
        let mut stores = BTreeMap::new();
        let mut max_seq = 0;
        for repo in repos {
            let store = WakeStore::load(paths.wakes_file(&repo))?;
            max_seq = max_seq.max(store.max_seq());
            stores.insert(repo, store);
        }
        let mut state = SupervisionState::new();
        state.seed_wake_seq(max_seq);
        Ok(Self {
            paths,
            state,
            stores,
            clients: BTreeMap::new(),
            offsets: BTreeMap::new(),
            seen_plans: BTreeSet::new(),
            away: BTreeMap::new(),
        })
    }

    /// Applies one event.
    pub fn handle(&mut self, event: SourceEvent) -> Result<()> {
        match event {
            SourceEvent::Filesystem(paths) => self.handle_paths(&paths),
            SourceEvent::Connected { repo, sender } => {
                self.clients.insert(repo, sender);
                self.deliver();
                Ok(())
            }
            SourceEvent::Message { message, .. } => {
                if let ProtocolMessage::Ack { wake } = message.message {
                    self.ack(&wake)?;
                }
                Ok(())
            }
            SourceEvent::Tick => {
                self.deliver();
                Ok(())
            }
            SourceEvent::Shutdown => Ok(()),
        }
    }

    /// Pending wakes for a repo, for tests and diagnostics.
    pub fn pending(&self, repo: &RepoId) -> usize {
        self.stores
            .get(repo)
            .map_or(0, |store| store.pending().count())
    }

    fn handle_paths(&mut self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            if let Some((repo, task)) = status_target(&self.paths, path) {
                self.read_status(&repo, &task, path)?;
            } else if let Some(repo) = plan_target(&self.paths, path) {
                if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                    self.plan_arrived(&repo, path)?;
                }
            } else if let Some(repo) = away_target(&self.paths, path) {
                self.away_changed(&repo, path.exists())?;
            }
        }
        Ok(())
    }

    fn read_status(&mut self, repo: &RepoId, task: &TaskId, path: &Path) -> Result<()> {
        let text = fs::read_to_string(path).unwrap_or_default();
        let start = self
            .offsets
            .get(path)
            .copied()
            .unwrap_or(0)
            .min(text.len() as u64) as usize;
        self.offsets.insert(path.to_path_buf(), text.len() as u64);

        for line in text[start..].lines().filter(|line| !line.trim().is_empty()) {
            let Ok(parsed) = StatusLine::parse(line) else {
                continue;
            };
            self.apply(Event::StatusLine {
                repo: repo.clone(),
                task: task.clone(),
                verb: parsed.verb,
                message: parsed.message,
                at: clock::now(),
            })?;
        }
        Ok(())
    }

    fn plan_arrived(&mut self, repo: &RepoId, path: &Path) -> Result<()> {
        let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
            return Ok(());
        };
        if !self.seen_plans.insert((repo.clone(), id.to_owned())) {
            return Ok(());
        }
        self.apply(Event::PlanArrived {
            repo: repo.clone(),
            plan: PlanId::new(id),
            at: clock::now(),
        })
    }

    fn away_changed(&mut self, repo: &RepoId, away: bool) -> Result<()> {
        if self.away.get(repo).copied() == Some(away) {
            return Ok(());
        }
        self.away.insert(repo.clone(), away);
        self.apply(Event::AwayChanged {
            repo: repo.clone(),
            away,
        })
    }

    fn apply(&mut self, event: Event) -> Result<()> {
        let (state, wakes) = bh_core::classify(std::mem::take(&mut self.state), event);
        self.state = state;
        for wake in wakes {
            self.record(wake)?;
        }
        Ok(())
    }

    fn record(&mut self, wake: Wake) -> Result<()> {
        self.store_mut(&wake.repo)?.append(&wake)
    }

    fn ack(&mut self, id: &WakeId) -> Result<()> {
        for store in self.stores.values_mut() {
            if store.is_pending(id) {
                store.mark_delivered(id, clock::now())?;
                return Ok(());
            }
        }
        Ok(())
    }

    fn deliver(&self) {
        for (repo, store) in &self.stores {
            let Some(client) = self.clients.get(repo) else {
                continue;
            };
            for wake in store.pending() {
                let envelope = Envelope::new(ProtocolMessage::Wake { wake: wake.clone() });
                let _ = client.send(envelope);
            }
        }
    }

    fn store_mut(&mut self, repo: &RepoId) -> Result<&mut WakeStore> {
        if !self.stores.contains_key(repo) {
            let store = WakeStore::load(self.paths.wakes_file(repo))?;
            self.stores.insert(repo.clone(), store);
        }
        self.stores
            .get_mut(repo)
            .context("wake store disappeared after insertion")
    }
}

fn status_target(paths: &Paths, path: &Path) -> Option<(RepoId, TaskId)> {
    let relative = path.strip_prefix(&paths.home).ok()?;
    let mut components = relative.components();
    let repo = components.next()?.as_os_str().to_str()?;
    if components.next()?.as_os_str() != "tasks" {
        return None;
    }
    let task = components.next()?.as_os_str().to_str()?;
    if components.next()?.as_os_str() != "status.log" {
        return None;
    }
    Some((RepoId::new(repo), TaskId::new(task)))
}

fn plan_target(paths: &Paths, path: &Path) -> Option<RepoId> {
    let relative = path.strip_prefix(&paths.home).ok()?;
    let mut components = relative.components();
    let repo = components.next()?.as_os_str().to_str()?;
    if components.next()?.as_os_str() != "plans" || components.next()?.as_os_str() != "inbox" {
        return None;
    }
    Some(RepoId::new(repo))
}

fn away_target(paths: &Paths, path: &Path) -> Option<RepoId> {
    let relative = path.strip_prefix(&paths.home).ok()?;
    let mut components = relative.components();
    let repo = components.next()?.as_os_str().to_str()?;
    if components.next()?.as_os_str() != "state" || components.next()?.as_os_str() != "away.json" {
        return None;
    }
    Some(RepoId::new(repo))
}
