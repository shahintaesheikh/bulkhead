//! Durable wake storage, one append-only `wakes.jsonl` per repo.
//!
//! Invariant 3: a wake is appended before delivery and a `Delivered` record is
//! appended only after the extension acknowledges it. Nothing is ever
//! rewritten, so a crash can only cause redelivery, never loss.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bh_core::{Timestamp, Wake, WakeId, WakeRecord};

/// In-memory view of one repo's `wakes.jsonl`.
#[derive(Debug)]
pub struct WakeStore {
    path: PathBuf,
    pending: BTreeMap<WakeId, Wake>,
    delivered: BTreeSet<WakeId>,
    max_seq: u64,
}

impl WakeStore {
    /// Loads a store, tolerating a truncated trailing line.
    pub fn load(path: PathBuf) -> Result<Self> {
        let mut store = Self {
            path,
            pending: BTreeMap::new(),
            delivered: BTreeSet::new(),
            max_seq: 0,
        };

        if store.path.exists() {
            let text = fs::read_to_string(&store.path)
                .with_context(|| format!("reading {}", store.path.display()))?;
            for line in text.lines().filter(|line| !line.trim().is_empty()) {
                match serde_json::from_str::<WakeRecord>(line) {
                    Ok(WakeRecord::Wake { wake }) => {
                        store.observe_seq(&wake.id);
                        store.pending.insert(wake.id.clone(), wake);
                    }
                    Ok(WakeRecord::Delivered { id, .. }) => {
                        store.delivered.insert(id);
                    }
                    Err(_) => continue,
                }
            }
        }

        let delivered = store.delivered.clone();
        store.pending.retain(|id, _| !delivered.contains(id));
        Ok(store)
    }

    /// Appends a wake and remembers it as pending.
    pub fn append(&mut self, wake: &Wake) -> Result<()> {
        self.observe_seq(&wake.id);
        self.pending.insert(wake.id.clone(), wake.clone());
        self.append_record(&WakeRecord::Wake { wake: wake.clone() })
    }

    /// Marks a wake delivered and appends the marker.
    pub fn mark_delivered(&mut self, id: &WakeId, at: Timestamp) -> Result<()> {
        self.delivered.insert(id.clone());
        self.pending.remove(id);
        self.append_record(&WakeRecord::Delivered { id: id.clone(), at })
    }

    /// Wakes still waiting for an ack.
    pub fn pending(&self) -> impl Iterator<Item = &Wake> {
        self.pending.values()
    }

    /// Whether a wake is pending.
    pub fn is_pending(&self, id: &WakeId) -> bool {
        self.pending.contains_key(id)
    }

    /// The highest wake sequence seen, for seeding `SupervisionState`.
    pub fn max_seq(&self) -> u64 {
        self.max_seq
    }

    fn append_record(&self, record: &WakeRecord) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("opening {}", self.path.display()))?;
        let line = serde_json::to_string(record).context("serializing wake record")?;
        writeln!(file, "{line}").context("appending wake record")?;
        Ok(())
    }

    fn observe_seq(&mut self, id: &WakeId) {
        if let Some(seq) = id
            .as_str()
            .strip_prefix("w-")
            .and_then(|rest| rest.parse::<u64>().ok())
        {
            self.max_seq = self.max_seq.max(seq);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bh_core::{RepoId, WakeKind};

    fn wake(seq: u64) -> Wake {
        Wake {
            id: WakeId::new(format!("w-{seq}")),
            repo: RepoId::from("bulkhead"),
            task: None,
            kind: WakeKind::TaskDone,
            summary: format!("wake {seq}"),
            created_at: Timestamp::from_secs(1_700_000_000 + seq),
        }
    }

    #[test]
    fn append_deliver_and_reload() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state/wakes.jsonl");

        let mut store = WakeStore::load(path.clone()).unwrap();
        store.append(&wake(1)).unwrap();
        store.append(&wake(2)).unwrap();
        store
            .mark_delivered(&WakeId::from("w-1"), Timestamp::from_secs(1))
            .unwrap();

        let reloaded = WakeStore::load(path).unwrap();
        assert_eq!(reloaded.max_seq(), 2);
        assert_eq!(reloaded.pending().count(), 1);
        assert!(reloaded.is_pending(&WakeId::from("w-2")));
        assert!(!reloaded.is_pending(&WakeId::from("w-1")));
    }

    #[test]
    fn truncated_trailing_line_is_tolerated() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("wakes.jsonl");
        fs::write(&path, "{\"record\":\"wake\",\"wake\":{").unwrap();
        let store = WakeStore::load(path).unwrap();
        assert_eq!(store.pending().count(), 0);
    }
}
