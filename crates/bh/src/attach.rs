//! `bh attach`: ensure the repo's herdr layout and orchestrator, then attach.
//!
//! `ensure` is pure orchestration over [`Herdr`] and the registry, so tests can
//! drive it against the scripted fake. The interactive `exec` happens only in
//! [`Herdr::attach`], from the CLI.

use anyhow::{Context, Result};
use bh_core::{PaneId, TabId, WorkspaceId};

use crate::herdr::Herdr;
use crate::orchestrator;
use crate::paths::BulkheadPaths;
use crate::registry;
use crate::repo::DetectedRepo;

/// Label of the tab the orchestrator runs in.
pub const ORCH_TAB_LABEL: &str = "orch";
/// Agent name herdr registers for the orchestrator.
pub const ORCH_AGENT_NAME: &str = "orch";
/// Agent kind for Pi.
pub const PI_KIND: &str = "pi";

/// What `ensure` had to do to reach a running orchestrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachPlan {
    /// The repo's workspace.
    pub workspace: WorkspaceId,
    /// The orchestrator tab.
    pub tab: TabId,
    /// The pane holding the orchestrator.
    pub pane: PaneId,
    /// The server was started by this call.
    pub started_server: bool,
    /// The workspace was created by this call.
    pub created_workspace: bool,
    /// The `orch` tab was created by this call.
    pub created_tab: bool,
    /// The orchestrator agent was started by this call.
    pub started_orchestrator: bool,
}

/// Ensures a running server, the repo's workspace, the `orch` tab and a live
/// orchestrator, returning what it did.
pub fn ensure(paths: &BulkheadPaths, herdr: &Herdr, repo: &DetectedRepo) -> Result<AttachPlan> {
    let mut registry = registry::load(paths)?;
    let entry = registry
        .repos
        .get(&repo.id)
        .with_context(|| format!("{} is not registered; run `bh init` first", repo.id))?
        .clone();

    // Keep the contract and extension in step with this binary.
    orchestrator::install(paths, &repo.id)?;
    let contract = orchestrator::contract_path(paths, &repo.id);
    let orchestrator_args = orchestrator::launch_args(&contract);

    let started_server = if herdr.server_running()? {
        false
    } else {
        herdr.start_server()?;
        true
    };

    let agent_dir = paths.agent_dir(&repo.id).to_string_lossy().into_owned();
    let env = [("PI_CODING_AGENT_DIR", agent_dir.as_str())];

    let workspaces = herdr.list_workspaces()?;
    let existing = entry
        .workspace
        .as_ref()
        .filter(|id| workspaces.iter().any(|w| &w.workspace_id == *id))
        .cloned()
        .or_else(|| {
            workspaces
                .iter()
                .find(|w| w.label == repo.id.as_str())
                .map(|w| w.workspace_id.clone())
        });

    let (workspace, created_workspace) = match existing {
        Some(id) => (id, false),
        None => {
            let created = herdr.create_workspace(&repo.root, repo.id.as_str(), &env)?;
            (created.workspace.workspace_id.clone(), true)
        }
    };

    let tabs = herdr.list_tabs(&workspace)?;
    let (orch_tab, created_tab) = match tabs.iter().find(|tab| tab.label == ORCH_TAB_LABEL) {
        Some(tab) => (tab.tab_id.clone(), false),
        None => {
            let created = herdr.create_tab(&workspace, &repo.root, ORCH_TAB_LABEL, &env)?;
            (created.tab.tab_id.clone(), true)
        }
    };

    let panes = herdr.list_panes(&workspace)?;
    let orch_panes: Vec<_> = panes
        .iter()
        .filter(|pane| pane.tab_id == orch_tab)
        .collect();
    let pane = orch_panes
        .first()
        .map(|pane| pane.pane_id.clone())
        .with_context(|| format!("tab {orch_tab} has no pane"))?;
    let running = orch_panes
        .iter()
        .any(|pane| pane.agent.as_deref() == Some(PI_KIND));
    let started_orchestrator = if running {
        false
    } else {
        herdr.start_agent(&pane, ORCH_AGENT_NAME, PI_KIND, &orchestrator_args)?;
        true
    };

    if entry.workspace.as_ref() != Some(&workspace) {
        if let Some(stored) = registry.repos.get_mut(&repo.id) {
            stored.workspace = Some(workspace.clone());
        }
        registry::save(paths, &registry)?;
    }

    Ok(AttachPlan {
        workspace,
        tab: orch_tab,
        pane,
        started_server,
        created_workspace,
        created_tab,
        started_orchestrator,
    })
}
