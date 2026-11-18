//! Small filesystem helpers with atomic writes.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `contents` to `path` atomically: temp file in the same directory,
/// then rename. Readers never observe a half-written file.
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;

    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("{} has no file name", path.display()))?;
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(".{name}.{}.{counter}.tmp", std::process::id()));

    fs::write(&temp, contents).with_context(|| format!("writing {}", temp.display()))?;
    fs::rename(&temp, path)
        .with_context(|| format!("renaming {} to {}", temp.display(), path.display()))?;
    Ok(())
}

/// Writes `contents` only when `path` does not already exist.
///
/// Returns whether the file was written. This is how `init` stays idempotent
/// without clobbering captain edits.
pub fn write_if_missing(path: &Path, contents: &str) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    write_atomic(path, contents)?;
    Ok(true)
}
