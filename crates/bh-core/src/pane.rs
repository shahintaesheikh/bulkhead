//! herdr pane agent lifecycle state.

use serde::{Deserialize, Serialize};

/// The agent state herdr reports for a pane.
///
/// Mirrors herdr's five lifecycle states. `Unknown` means an agent is present
/// but herdr cannot classify it confidently; it does not mean the pane is
/// empty or the crewmate is finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PaneState {
    /// The agent is ready for input.
    Idle,
    /// The agent is running a turn.
    Working,
    /// herdr recognized an approval or question dialog.
    Blocked,
    /// The agent settled after a successful turn.
    Done,
    /// An agent is present but herdr cannot classify it.
    Unknown,
}

impl PaneState {
    /// The herdr token for this state.
    pub fn as_token(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Unknown => "unknown",
        }
    }
}
