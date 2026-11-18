//! herdr adapter.
//!
//! Every call shells out to the `herdr` CLI with an argument array. Workspaces,
//! tabs and panes are addressed by id, never by label (invariant 4). When
//! bulkhead starts the server itself it does so with a cleared environment
//! (invariant 5), so an ambient `PI_CODING_AGENT_DIR` can never leak into the
//! shared server.
//!
//! Command shapes and result keys were confirmed against `herdr 0.9.5`,
//! protocol 22 (`herdr api schema` and `herdr status`).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use bh_core::{PaneId, TabId, WorkspaceId};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// The shared herdr session bulkhead owns.
pub const SESSION: &str = "bulkhead";

/// A thin client over the `herdr` CLI.
#[derive(Debug, Clone)]
pub struct Herdr {
    bin: PathBuf,
    session: String,
    child_env: Vec<(String, String)>,
}

impl Herdr {
    /// Builds a client for a given binary and session.
    pub fn new(bin: impl Into<PathBuf>, session: impl Into<String>) -> Self {
        Self {
            bin: bin.into(),
            session: session.into(),
            child_env: Vec::new(),
        }
    }

    /// Builds the default client, honoring `BULKHEAD_HERDR_BIN`.
    pub fn from_env() -> Self {
        let bin = std::env::var_os("BULKHEAD_HERDR_BIN")
            .map_or_else(|| PathBuf::from("herdr"), PathBuf::from);
        Self::new(bin, SESSION)
    }

    /// Adds an explicit variable to every child process.
    ///
    /// This is how tests point the scripted fake at its state directory; it is
    /// never inherited from the parent environment.
    pub fn with_child_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.child_env.push((key.into(), value.into()));
        self
    }

    /// The session name.
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Whether the session's server is running.
    pub fn server_running(&self) -> Result<bool> {
        let output = self
            .command()
            .args(["status", "server"])
            .output()
            .context("running herdr status server")?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(output.status.success() && stdout.contains("status: running"))
    }

    /// Starts the server with a clean environment.
    ///
    /// `PATH` and `HOME` are the minimum herdr needs to find its binary and
    /// config; only [`Herdr::with_child_env`] values are added beyond that.
    pub fn start_server(&self) -> Result<()> {
        let mut command = Command::new(&self.bin);
        command.arg("--session").arg(&self.session).arg("server");
        command.env_clear();
        if let Some(path) = std::env::var_os("PATH") {
            command.env("PATH", path);
        }
        if let Some(home) = std::env::var_os("HOME") {
            command.env("HOME", home);
        }
        for (key, value) in &self.child_env {
            command.env(key, value);
        }

        let status = command.status().context("starting herdr server")?;
        if !status.success() {
            bail!("herdr server exited with {status}");
        }
        Ok(())
    }

    /// Lists all workspaces.
    pub fn list_workspaces(&self) -> Result<Vec<WorkspaceInfo>> {
        let result: WorkspaceListResult = self.run_json(&owned(&["workspace", "list"]))?;
        Ok(result.workspaces)
    }

    /// Creates a workspace and returns its root tab and pane.
    pub fn create_workspace(
        &self,
        cwd: &Path,
        label: &str,
        env: &[(&str, &str)],
    ) -> Result<CreatedWorkspace> {
        let mut args = owned(&["workspace", "create", "--no-focus", "--cwd"]);
        args.push(cwd.to_string_lossy().into_owned());
        args.push("--label".to_owned());
        args.push(label.to_owned());
        push_env(&mut args, env);

        let result: CreateWorkspaceResult = self.run_json(&args)?;
        Ok(CreatedWorkspace {
            workspace: result.workspace,
            tab: result.tab,
            root_pane: result.root_pane,
        })
    }

    /// Lists tabs in a workspace.
    pub fn list_tabs(&self, workspace: &WorkspaceId) -> Result<Vec<TabInfo>> {
        let result: TabListResult =
            self.run_json(&owned(&["tab", "list", "--workspace", workspace.as_str()]))?;
        Ok(result.tabs)
    }

    /// Creates a tab and returns its root pane.
    pub fn create_tab(
        &self,
        workspace: &WorkspaceId,
        cwd: &Path,
        label: &str,
        env: &[(&str, &str)],
    ) -> Result<CreatedTab> {
        let mut args = owned(&[
            "tab",
            "create",
            "--no-focus",
            "--workspace",
            workspace.as_str(),
            "--cwd",
        ]);
        args.push(cwd.to_string_lossy().into_owned());
        args.push("--label".to_owned());
        args.push(label.to_owned());
        push_env(&mut args, env);

        let result: CreateTabResult = self.run_json(&args)?;
        Ok(CreatedTab {
            tab: result.tab,
            root_pane: result.root_pane,
        })
    }

    /// Lists panes in a workspace.
    pub fn list_panes(&self, workspace: &WorkspaceId) -> Result<Vec<PaneInfo>> {
        let result: PaneListResult =
            self.run_json(&owned(&["pane", "list", "--workspace", workspace.as_str()]))?;
        Ok(result.panes)
    }

    /// Starts a recognized agent in an existing pane.
    pub fn start_agent(
        &self,
        pane: &PaneId,
        name: &str,
        kind: &str,
        extra: &[String],
    ) -> Result<()> {
        let mut args = owned(&[
            "agent",
            "start",
            name,
            "--kind",
            kind,
            "--pane",
            pane.as_str(),
        ]);
        if !extra.is_empty() {
            args.push("--".to_owned());
            args.extend(extra.iter().cloned());
        }
        self.run_ok(&args)
    }

    /// Sends a message to the agent occupying a pane.
    pub fn prompt_agent(&self, pane: &PaneId, text: &str) -> Result<()> {
        self.run_ok(&owned(&["agent", "prompt", pane.as_str(), text]))
    }

    /// Closes a tab and every pane in it.
    pub fn close_tab(&self, tab: &TabId) -> Result<()> {
        self.run_ok(&owned(&["tab", "close", tab.as_str()]))
    }

    /// Replaces this process with the herdr TUI attached to the session.
    ///
    /// Only returns on failure; on success the process image is replaced.
    pub fn attach(&self) -> Result<()> {
        use std::os::unix::process::CommandExt;

        let error = Command::new(&self.bin)
            .arg("--session")
            .arg(&self.session)
            .exec();
        Err(error).context("attaching to herdr session")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.bin);
        command.arg("--session").arg(&self.session);
        command.envs(self.child_env.iter().map(|(k, v)| (k, v)));
        command
    }

    fn run_json<T: DeserializeOwned>(&self, args: &[String]) -> Result<T> {
        let output = self
            .command()
            .args(args)
            .output()
            .with_context(|| format!("running herdr {}", args.join(" ")))?;

        if !output.status.success() {
            bail!(
                "herdr {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        let value: Value = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("parsing herdr {} response", args.join(" ")))?;
        if let Some(error) = value.get("error") {
            bail!("herdr {} returned an error: {error}", args.join(" "));
        }

        let result = value
            .get("result")
            .cloned()
            .with_context(|| format!("herdr {} response has no result", args.join(" ")))?;
        serde_json::from_value(result)
            .with_context(|| format!("decoding herdr {} result", args.join(" ")))
    }

    fn run_ok(&self, args: &[String]) -> Result<()> {
        let output = self
            .command()
            .args(args)
            .output()
            .with_context(|| format!("running herdr {}", args.join(" ")))?;
        if !output.status.success() {
            bail!(
                "herdr {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }
}

/// One workspace, as herdr reports it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WorkspaceInfo {
    /// Stable workspace id (`w1`).
    pub workspace_id: WorkspaceId,
    /// Human label.
    #[serde(default)]
    pub label: String,
    /// Active tab id, when known.
    #[serde(default)]
    pub active_tab_id: Option<TabId>,
}

/// One tab, as herdr reports it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TabInfo {
    /// Stable tab id (`w1:t1`).
    pub tab_id: TabId,
    /// Owning workspace.
    pub workspace_id: WorkspaceId,
    /// Human label.
    #[serde(default)]
    pub label: String,
}

/// One pane, as herdr reports it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PaneInfo {
    /// Stable pane id (`w1:p1`).
    pub pane_id: PaneId,
    /// Owning tab.
    pub tab_id: TabId,
    /// Owning workspace.
    pub workspace_id: WorkspaceId,
    /// Optional pane label.
    #[serde(default)]
    pub label: Option<String>,
    /// Recognized agent occupying the pane, if any.
    #[serde(default)]
    pub agent: Option<String>,
}

/// Result of creating a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedWorkspace {
    /// The new workspace.
    pub workspace: WorkspaceInfo,
    /// Its root tab.
    pub tab: TabInfo,
    /// That tab's root pane.
    pub root_pane: PaneInfo,
}

/// Result of creating a tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedTab {
    /// The new tab.
    pub tab: TabInfo,
    /// Its root pane.
    pub root_pane: PaneInfo,
}

#[derive(Deserialize)]
struct WorkspaceListResult {
    workspaces: Vec<WorkspaceInfo>,
}

#[derive(Deserialize)]
struct TabListResult {
    tabs: Vec<TabInfo>,
}

#[derive(Deserialize)]
struct PaneListResult {
    panes: Vec<PaneInfo>,
}

#[derive(Deserialize)]
struct CreateWorkspaceResult {
    workspace: WorkspaceInfo,
    tab: TabInfo,
    root_pane: PaneInfo,
}

#[derive(Deserialize)]
struct CreateTabResult {
    tab: TabInfo,
    root_pane: PaneInfo,
}

fn owned(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_owned()).collect()
}

fn push_env(args: &mut Vec<String>, env: &[(&str, &str)]) {
    for (key, value) in env {
        args.push("--env".to_owned());
        args.push(format!("{key}={value}"));
    }
}
