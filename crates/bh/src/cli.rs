//! Command-line parsing and dispatch.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::doctor::{self, Level};
use crate::init;
use crate::paths::BulkheadPaths;
use crate::repo;

/// `bh`: the bulkhead command-line interface.
#[derive(Debug, Parser)]
#[command(name = "bh", version, about = "bulkhead command-line interface")]
pub struct Cli {
    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// Available `bh` subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Register a repo and build its bulkhead home and agent folder.
    Init {
        /// A path inside the repo (default: the current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Check the registry, repo homes and agent folders.
    Doctor,
}

/// Parses arguments and runs the selected command.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { path } => run_init(&path),
        Command::Doctor => run_doctor(),
    }
}

fn run_init(path: &Path) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(path)?;
    let report = init::init(&paths, &repo)?;

    println!("registered {} at {}", report.id, report.home.display());
    if report.already_registered {
        println!("  (already registered; layout refreshed)");
    }
    for (name, outcome) in &report.links {
        println!("  {name}: {outcome:?}");
    }

    Ok(())
}

fn run_doctor() -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let findings = doctor::doctor(&paths)?;

    let mut errors = 0usize;
    for finding in &findings {
        if finding.level == Level::Error {
            errors += 1;
        }
        let scope = match &finding.repo {
            Some(repo) => format!("{repo}: "),
            None => String::new(),
        };
        println!("[{}] {scope}{}", label(finding.level), finding.message);
    }

    if errors > 0 {
        anyhow::bail!("{errors} error(s) found");
    }
    Ok(())
}

fn label(level: Level) -> &'static str {
    match level {
        Level::Ok => "ok",
        Level::Warn => "warn",
        Level::Error => "error",
    }
}
