//! Command-line parsing and dispatch.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use bh_core::{PlanId, TaskId, TriageState};
use clap::{Parser, Subcommand};

use crate::attach;
use crate::away;
use crate::backlog;
use crate::delivery::{self, Landers};
use crate::doctor::{self, Level};
use crate::ghaxi::GhAxi;
use crate::herdr::Herdr;
use crate::init;
use crate::lavish::LavishAxi;
use crate::no_mistakes::NoMistakes;
use crate::paths::BulkheadPaths;
use crate::plan;
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
    /// Merge an approved task and tear it down.
    Land {
        /// Task id.
        task_id: String,
        /// Confirm the captain approved this merge.
        #[arg(long)]
        approve: bool,
    },
    /// Add a plan to the inbox, or triage plans.
    Plan {
        /// Path to the plan file to ingest.
        path: Option<PathBuf>,
        /// List plans and their triage state.
        #[arg(long)]
        list: bool,
        /// Accept a plan by id.
        #[arg(long)]
        accept: Option<String>,
        /// Decline a plan by id.
        #[arg(long)]
        decline: Option<String>,
        /// Supersede a plan by id.
        #[arg(long)]
        supersede: Option<String>,
        /// Reason for a decline or supersede.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Create a backlog item from an ask.
    Task {
        /// The ask. Omit with --list.
        ask: Option<String>,
        /// List backlog items.
        #[arg(long)]
        list: bool,
    },
    /// Toggle away-mode; `off` prints the return brief.
    Away {
        /// `on` or `off`.
        mode: String,
    },
    /// Ensure the herdr workspace and orchestrator exist, without attaching.
    Ensure {
        /// A path inside the repo (default: the current directory).
        #[arg(default_value = ".")]
        path: PathBuf,
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
        Some(Command::Land { task_id, approve }) => run_land(&task_id, approve),
        Some(Command::Plan {
            path,
            list,
            accept,
            decline,
            supersede,
            reason,
        }) => run_plan(path.as_deref(), list, accept, decline, supersede, reason),
        Some(Command::Task { ask, list }) => run_task(ask.as_deref(), list),
        Some(Command::Away { mode }) => run_away(&mode),
        Some(Command::Ensure { path }) => run_ensure(&path),
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
        bail!("{errors} error(s) found");
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

fn run_land(task_id: &str, approve: bool) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(Path::new("."))?;
    let herdr = Herdr::from_env();
    let treehouse = Treehouse::from_env();
    let ghaxi = GhAxi::from_env();
    let no_mistakes = NoMistakes::from_env();
    let landers = Landers {
        herdr: &herdr,
        treehouse: &treehouse,
        ghaxi: &ghaxi,
        no_mistakes: &no_mistakes,
    };
    let report = delivery::land(&paths, &landers, &repo.id, &TaskId::from(task_id), approve)?;
    println!(
        "landed {} ({:?}) -> {}",
        report.task_id, report.mode, report.reference
    );
    Ok(())
}

fn run_plan(
    path: Option<&Path>,
    list: bool,
    accept: Option<String>,
    decline: Option<String>,
    supersede: Option<String>,
    reason: Option<String>,
) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(Path::new("."))?;

    if list {
        for plan in plan::list(&paths, &repo.id)? {
            println!("{} [{}] {}", plan.id, plan.state.as_token(), plan.title);
        }
        return Ok(());
    }

    if let Some(id) = accept {
        let plan = plan::set_state(
            &paths,
            &repo.id,
            &PlanId::from(id),
            TriageState::Accepted,
            None,
        )?;
        println!("accepted {} ({})", plan.id, plan.file);
        return Ok(());
    }
    if let Some(id) = decline {
        let plan = plan::set_state(
            &paths,
            &repo.id,
            &PlanId::from(id),
            TriageState::Declined,
            reason,
        )?;
        println!("declined {} ({})", plan.id, plan.file);
        return Ok(());
    }
    if let Some(id) = supersede {
        let plan = plan::set_state(
            &paths,
            &repo.id,
            &PlanId::from(id),
            TriageState::Superseded,
            reason,
        )?;
        println!("superseded {} ({})", plan.id, plan.file);
        return Ok(());
    }

    if let Some(path) = path {
        let lavish = LavishAxi::from_env();
        let plan = plan::ingest(&paths, &repo.id, path, Some(&lavish))?;
        println!("ingested {} into the inbox ({})", plan.id, plan.title);
        if let Some(url) = plan.review_url {
            println!("review: {url}");
        }
        return Ok(());
    }

    bail!("nothing to do: pass a plan path, or --list / --accept / --decline / --supersede")
}

fn run_task(ask: Option<&str>, list: bool) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(Path::new("."))?;

    if list {
        for item in backlog::list(&paths, &repo.id)? {
            println!("{} [{}] {}", item.id, item.state, item.title);
        }
        return Ok(());
    }

    let ask = ask.context("provide an ask, or pass --list")?;
    let id = backlog::create(&paths, &repo.id, ask)?;
    println!("queued {id}");
    Ok(())
}

fn run_away(mode: &str) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(Path::new("."))?;
    match mode {
        "on" => {
            away::set(&paths, &repo.id, true)?;
            println!("away-mode on for {}", repo.id);
        }
        "off" => {
            away::set(&paths, &repo.id, false)?;
            print!("{}", away::brief(&paths, &repo.id)?);
        }
        other => bail!("away expects `on` or `off`, got {other:?}"),
    }
    Ok(())
}

fn run_ensure(path: &Path) -> Result<()> {
    let paths = BulkheadPaths::discover()?;
    let repo = repo::detect(path)?;
    let herdr = Herdr::from_env();
    let plan = attach::ensure(&paths, &herdr, &repo)?;
    println!(
        "ensured {} (workspace {}, tab {}, pane {})",
        repo.id, plan.workspace, plan.tab, plan.pane
    );
    Ok(())
}

fn label(level: Level) -> &'static str {
    match level {
        Level::Ok => "ok",
        Level::Warn => "warn",
        Level::Error => "error",
    }
}
