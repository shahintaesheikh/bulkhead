//! lavish-axi adapter: HTML plan review.
//!
//! The CLI shape is assumed because lavish-axi is not installed while bulkhead
//! is built; tests drive `tests/fixtures/fake-lavish`. The contract:
//!
//! - `lavish-axi publish <path>` prints a review URL.
//! - `lavish-axi poll <review>` prints reviewer feedback.
//!
//! `BULKHEAD_LAVISH_BIN` overrides the program; otherwise `npx -y lavish-axi`.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// A thin client over the `lavish-axi` CLI.
#[derive(Debug, Clone)]
pub struct LavishAxi {
    program: String,
    prefix: Vec<String>,
    child_env: Vec<(String, String)>,
}

impl LavishAxi {
    /// Builds a client around a program and its leading arguments.
    pub fn new(program: impl Into<String>, prefix: &[&str]) -> Self {
        Self {
            program: program.into(),
            prefix: prefix.iter().map(|arg| (*arg).to_owned()).collect(),
            child_env: Vec::new(),
        }
    }

    /// Builds the default client, honoring `BULKHEAD_LAVISH_BIN`.
    pub fn from_env() -> Self {
        match std::env::var("BULKHEAD_LAVISH_BIN") {
            Ok(bin) => Self::new(bin, &[]),
            Err(_) => Self::new("npx", &["-y", "lavish-axi"]),
        }
    }

    /// Adds an explicit variable to every child process (used by tests).
    pub fn with_child_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.child_env.push((key.into(), value.into()));
        self
    }

    /// Publishes an HTML plan and returns its review URL.
    pub fn publish(&self, path: &Path) -> Result<String> {
        self.run_capture(&["publish", &path.to_string_lossy()])
    }

    /// Polls a review for feedback.
    pub fn poll(&self, review: &str) -> Result<String> {
        self.run_capture(&["poll", review])
    }

    fn run_capture(&self, args: &[&str]) -> Result<String> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.prefix)
            .args(args)
            .envs(self.child_env.iter().map(|(key, value)| (key, value)));
        let output = command
            .output()
            .with_context(|| format!("running lavish-axi {}", args.join(" ")))?;
        if !output.status.success() {
            bail!(
                "lavish-axi {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
}
