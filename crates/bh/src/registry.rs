//! Reading and writing `~/.bulkhead/registry.toml`.

use std::fs;

use anyhow::{Context, Result};
use bh_core::Registry;

use crate::fsx::write_atomic;
use crate::paths::BulkheadPaths;

/// Loads the registry, or an empty one when the file does not exist yet.
pub fn load(paths: &BulkheadPaths) -> Result<Registry> {
    let path = paths.registry_file();
    if !path.exists() {
        return Ok(Registry::default());
    }

    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Writes the registry atomically.
pub fn save(paths: &BulkheadPaths, registry: &Registry) -> Result<()> {
    let text = toml::to_string_pretty(registry).context("serializing registry")?;
    write_atomic(&paths.registry_file(), &text)
}
