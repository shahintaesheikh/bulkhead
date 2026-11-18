//! Discovering the repo a `bh` command runs inside.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use bh_core::RepoId;

/// A repo found on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedRepo {
    /// Absolute path to the git root.
    pub root: PathBuf,
    /// Origin remote URL, if any.
    pub origin: Option<String>,
    /// Slugified id derived from the root directory name.
    pub id: RepoId,
}

/// Finds the git root at or above `start` and derives a repo id.
pub fn detect(start: &Path) -> Result<DetectedRepo> {
    let root = PathBuf::from(git(start, &["rev-parse", "--show-toplevel"])?.trim());
    let origin = git(&root, &["remote", "get-url", "origin"])
        .ok()
        .map(|url| url.trim().to_owned());

    let dir_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("repo root {} has no directory name", root.display()))?;
    let id = slugify(dir_name)
        .with_context(|| format!("cannot derive a repo id from directory name {dir_name:?}"))?;

    Ok(DetectedRepo { root, origin, id })
}

/// Lowercases a name and reduces runs of non-alphanumerics to `-`.
pub fn slugify(name: &str) -> Option<RepoId> {
    let mut slug = String::new();
    let mut pending_dash = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch.to_ascii_lowercase());
            pending_dash = false;
        } else {
            pending_dash = true;
        }
    }

    if slug.is_empty() {
        None
    } else {
        Some(RepoId::new(slug))
    }
}

/// Runs git with an argument array (never a shell string) and returns stdout.
fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .with_context(|| format!("running git {} in {}", args.join(" "), dir.display()))?;

    if !output.status.success() {
        bail!(
            "git {} failed in {}: {}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_reduces_separators_and_case() {
        assert_eq!(slugify("bulkhead").unwrap().as_str(), "bulkhead");
        assert_eq!(slugify("Acme App").unwrap().as_str(), "acme-app");
        assert_eq!(slugify("my__repo!!").unwrap().as_str(), "my-repo");
        assert_eq!(slugify("  ---  "), None);
        assert_eq!(slugify(""), None);
    }
}
