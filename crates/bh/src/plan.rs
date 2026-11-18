//! Plan ingress and triage.
//!
//! Plans live under `<repo-home>/plans/{inbox,accepted,declined}/`, one file
//! plus a `<id>.meta.toml` sidecar. Triage moves both between buckets; the
//! sidecar is the source of truth for state.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use bh_core::{Plan, PlanId, RepoId, TriageState};

use crate::clock;
use crate::fsx;
use crate::lavish::LavishAxi;
use crate::paths::BulkheadPaths;
use crate::repo::slug_text;

const BUCKETS: [&str; 3] = ["inbox", "accepted", "declined"];

/// Reads a plan file into the inbox.
pub fn ingest(
    paths: &BulkheadPaths,
    repo: &RepoId,
    source: &Path,
    lavish: Option<&LavishAxi>,
) -> Result<Plan> {
    let text =
        fs::read_to_string(source).with_context(|| format!("reading {}", source.display()))?;
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("plan");
    let extension = source
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("md")
        .to_owned();

    let id = unique_id(
        paths,
        repo,
        slug_text(stem).unwrap_or_else(|| "plan".to_owned()),
    );
    let title = first_heading(&text).unwrap_or_else(|| stem.replace(['-', '_'], " "));
    let file = format!("{id}.{extension}");

    let bucket = paths.repo_home(repo).join("plans").join("inbox");
    fs::create_dir_all(&bucket).with_context(|| format!("creating {}", bucket.display()))?;
    fsx::write_atomic(&bucket.join(&file), &text)?;

    let review_url = if extension.eq_ignore_ascii_case("html") {
        lavish.and_then(|client| client.publish(source).ok())
    } else {
        None
    };

    let plan = Plan {
        id,
        repo: repo.clone(),
        title,
        state: TriageState::Inbox,
        file,
        source: source.display().to_string(),
        review_url,
        reason: None,
        created_at: clock::now(),
    };
    write_meta(&bucket, &plan)?;
    Ok(plan)
}

/// Moves a plan to the bucket for `state` and records the reason.
pub fn set_state(
    paths: &BulkheadPaths,
    repo: &RepoId,
    id: &PlanId,
    state: TriageState,
    reason: Option<String>,
) -> Result<Plan> {
    let (bucket, mut plan) = find(paths, repo, id)?;
    let target = paths.repo_home(repo).join("plans").join(state.bucket());

    if bucket != target {
        fs::create_dir_all(&target).with_context(|| format!("creating {}", target.display()))?;
        fs::rename(bucket.join(&plan.file), target.join(&plan.file))
            .with_context(|| format!("moving {}", plan.file))?;
        fs::rename(
            bucket.join(format!("{id}.meta.toml")),
            target.join(format!("{id}.meta.toml")),
        )
        .with_context(|| format!("moving {id}.meta.toml"))?;
    }

    plan.state = state;
    plan.reason = reason;
    write_meta(&target, &plan)?;
    Ok(plan)
}

/// Lists every plan in the repo.
pub fn list(paths: &BulkheadPaths, repo: &RepoId) -> Result<Vec<Plan>> {
    let mut plans = Vec::new();
    for bucket in BUCKETS {
        let directory = paths.repo_home(repo).join("plans").join(bucket);
        if !directory.is_dir() {
            continue;
        }
        for entry in
            fs::read_dir(&directory).with_context(|| format!("reading {}", directory.display()))?
        {
            let path = entry?.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".meta.toml"))
            {
                plans.push(read_meta(&path)?);
            }
        }
    }
    plans.sort_by_key(|plan| plan.created_at);
    Ok(plans)
}

fn find(paths: &BulkheadPaths, repo: &RepoId, id: &PlanId) -> Result<(PathBuf, Plan)> {
    for bucket in BUCKETS {
        let directory = paths.repo_home(repo).join("plans").join(bucket);
        let meta = directory.join(format!("{id}.meta.toml"));
        if meta.exists() {
            return Ok((directory, read_meta(&meta)?));
        }
    }
    bail!("plan {id} not found")
}

fn unique_id(paths: &BulkheadPaths, repo: &RepoId, base: String) -> PlanId {
    let mut candidate = base.clone();
    let mut suffix = 2;
    while exists(paths, repo, &candidate) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    PlanId::new(candidate)
}

fn exists(paths: &BulkheadPaths, repo: &RepoId, id: &str) -> bool {
    BUCKETS.iter().any(|bucket| {
        paths
            .repo_home(repo)
            .join("plans")
            .join(bucket)
            .join(format!("{id}.meta.toml"))
            .exists()
    })
}

fn write_meta(bucket: &Path, plan: &Plan) -> Result<()> {
    let text = toml::to_string_pretty(plan).context("serializing plan metadata")?;
    fsx::write_atomic(&bucket.join(format!("{}.meta.toml", plan.id)), &text)
}

fn read_meta(path: &Path) -> Result<Plan> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn first_heading(text: &str) -> Option<String> {
    text.lines()
        .find(|line| line.trim_start().starts_with('#'))
        .map(|line| line.trim_start_matches('#').trim().to_owned())
        .filter(|title| !title.is_empty())
}
