//! gh-axi adapter.
//!
//! gh-axi is an agent-oriented wrapper over the GitHub CLI. Its real command
//! surface was read with `npx -y gh-axi pr --help` while building this unit:
//! `pr create|merge|checks`, `stack ...`. `BULKHEAD_GHAXI_BIN` overrides the
//! program; otherwise it runs `npx -y gh-axi`.

use std::process::Command;

use anyhow::{Context, Result, bail};

/// A thin client over the `gh-axi` CLI.
#[derive(Debug, Clone)]
pub struct GhAxi {
    program: String,
    prefix: Vec<String>,
    child_env: Vec<(String, String)>,
}

impl GhAxi {
    /// Builds a client around a program and its leading arguments.
    pub fn new(program: impl Into<String>, prefix: &[&str]) -> Self {
        Self {
            program: program.into(),
            prefix: prefix.iter().map(|arg| (*arg).to_owned()).collect(),
            child_env: Vec::new(),
        }
    }

    /// Builds the default client, honoring `BULKHEAD_GHAXI_BIN`.
    pub fn from_env() -> Self {
        match std::env::var("BULKHEAD_GHAXI_BIN") {
            Ok(bin) => Self::new(bin, &[]),
            Err(_) => Self::new("npx", &["-y", "gh-axi"]),
        }
    }

    /// Adds an explicit variable to every child process (used by tests).
    pub fn with_child_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.child_env.push((key.into(), value.into()));
        self
    }

    /// Merges a PR.
    pub fn merge(&self, number: &str, method: &str) -> Result<()> {
        self.run_ok(&["pr", "merge", number, "--method", method, "--delete-branch"])
    }

    /// Opens a PR and returns its URL.
    pub fn create(
        &self,
        title: &str,
        body_file: &str,
        base: Option<&str>,
        head: Option<&str>,
        draft: bool,
    ) -> Result<String> {
        let mut args = vec!["pr", "create", "--title", title, "--body-file", body_file];
        if let Some(base) = base {
            args.push("--base");
            args.push(base);
        }
        if let Some(head) = head {
            args.push("--head");
            args.push(head);
        }
        if draft {
            args.push("--draft");
        }
        self.run_capture(&args)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.prefix);
        command.envs(self.child_env.iter().map(|(key, value)| (key, value)));
        command
    }

    fn run_ok(&self, args: &[&str]) -> Result<()> {
        let output = self
            .command()
            .args(args)
            .output()
            .with_context(|| format!("running gh-axi {}", args.join(" ")))?;
        if !output.status.success() {
            bail!(
                "gh-axi {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }

    fn run_capture(&self, args: &[&str]) -> Result<String> {
        let output = self
            .command()
            .args(args)
            .output()
            .with_context(|| format!("running gh-axi {}", args.join(" ")))?;
        if !output.status.success() {
            bail!(
                "gh-axi {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
}
