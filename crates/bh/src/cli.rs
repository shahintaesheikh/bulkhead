//! Command-line parsing and dispatch.

use std::path::{Path, PathBuf};

use anyhow::Result;
use bh_core::TaskId;
use clap::{Parser, Subcommand};

use crate::attach;
use crate::doctor::{self, Level};
use crate::herdr::Herdr;
use crate::init;
use crate::paths::BulkheadPaths;
use crate::repo;
use crate::task;
use crate::treehouse::Treehouse;

/// `bh`: the bulkhead command-line interface.
#[derive(Debug, Parser)]
#[command(name = "bh", version, about = "bulkhead command-line interface")]
pub struct Cli {
    /// The command to run. Defaults to `attach` for the current repo.
    #[command(subcommand)]
    pub command: Option<Command>,
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
    /// Attach to a repo's herdr workspace, starting the orchestrator.
    Attach {
        /// A path inside the repo (default: the current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Spawn a crewmate for a backlog task.
    Spawn {
        /// Task id, matching a backlog item.
        task_id: String,
    },
    /// Send a message to a crewmate.
    Send {
        /// Task id.
        task_id: String,
        /// Message to deliver.
        message: String,
    },
    /// Show task status.
    Status {
        /// Task id; omit to list every task.
        task_id: Option<String>,
    },
    /// Tear a task down and return its worktree.
    Teardown {
        /// Task id.
        task_id: String,
    },
}

/// Parses arguments and runs the selected command.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Init { path }) => run_init(&path),
        Some(Command::Doctor) => run_doctor(),
        Some(Command::Attach { path }) => run_attach(&path),
        Some(Command::Spawn { task_id }) => run_spawn(&task_id),
        Some(Command::Send { task_id, message }) => run_send(&task_id, &message),
        Some(Command::Status { task_id }) => run_status(task_id.as_deref()),
        Some(Command::Teardown { task_id }) => run_teardown(&task_id),
        None => run_attach(Path::new(".")),
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

fn run_attach(path: &Path) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(path)?;
    let herdr = Herdr::from_env();
    let plan = attach::ensure(&paths, &herdr, &repo)?;

    if plan.started_server {
        println!("started herdr session {}", herdr.session());
    }
    if plan.created_workspace {
        println!("created herdr workspace {} for {}", plan.workspace, repo.id);
    }
    if plan.created_tab {
        println!("created {} tab {}", attach::ORCH_TAB_LABEL, plan.tab);
    }
    if plan.started_orchestrator {
        println!("started orchestrator in pane {}", plan.pane);
    }

    herdr.attach()
}

fn run_spawn(task_id: &str) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(Path::new("."))?;
    let herdr = Herdr::from_env();
    let treehouse = Treehouse::from_env();
    let report = task::spawn(&paths, &herdr, &treehouse, &repo, &TaskId::from(task_id))?;
    println!(
        "spawned {} in {} (tab {}, pane {})",
        report.task_id,
        report.worktree.display(),
        report.tab,
        report.pane
    );
    Ok(())
}

fn run_send(task_id: &str, message: &str) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(Path::new("."))?;
    let herdr = Herdr::from_env();
    task::send(&herdr, &paths, &repo.id, &TaskId::from(task_id), message)?;
    println!("sent to {task_id}");
    Ok(())
}

fn run_status(task_id: Option<&str>) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(Path::new("."))?;
    let task_id = task_id.map(TaskId::from);
    print!("{}", task::status(&paths, &repo.id, task_id.as_ref())?);
    Ok(())
}

fn run_teardown(task_id: &str) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(Path::new("."))?;
    let herdr = Herdr::from_env();
    let treehouse = Treehouse::from_env();
    task::teardown(&herdr, &treehouse, &paths, &repo.id, &TaskId::from(task_id))?;
    println!("tore down {task_id}");
    Ok(())
}

fn label(level: Level) -> &'static str {
    match level {
        Level::Ok => "ok",
        Level::Warn => "warn",
        Level::Error => "error",
    }
}
