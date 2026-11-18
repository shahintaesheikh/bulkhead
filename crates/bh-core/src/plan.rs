//! Plans and their triage state.

use serde::{Deserialize, Serialize};

use crate::ids::{PlanId, RepoId};
use crate::time::Timestamp;

/// Where a plan sits in triage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TriageState {
    /// Newly arrived, not yet read.
    Inbox,
    /// Turned into backlog items.
    Accepted,
    /// Needs a decision before it can be accepted or declined.
    NeedsReview,
    /// Rejected with a reason.
    Declined,
    /// Replaced by a newer plan.
    Superseded,
}

impl TriageState {
    /// The on-disk token.
    pub fn as_token(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Accepted => "accepted",
            Self::NeedsReview => "needs-review",
            Self::Declined => "declined",
            Self::Superseded => "superseded",
        }
    }

    /// The `plans/` bucket this state lives in.
    pub fn bucket(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Declined | Self::Superseded => "declined",
            Self::Inbox | Self::NeedsReview => "inbox",
        }
    }

    /// Whether the plan is still waiting on someone.
    pub fn is_open(self) -> bool {
        matches!(self, Self::Inbox | Self::NeedsReview)
    }
}

/// One plan's metadata (the `.meta.toml` sidecar).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    /// Plan id.
    pub id: PlanId,
    /// Owning repo.
    pub repo: RepoId,
    /// Human title.
    pub title: String,
    /// Current triage state.
    pub state: TriageState,
    /// Stored file name, relative to its bucket.
    pub file: String,
    /// Where the plan came from.
    #[serde(default)]
    pub source: String,
    /// lavish-axi review URL, for HTML plans.
    #[serde(default)]
    pub review_url: Option<String>,
    /// Why it was declined or superseded.
    #[serde(default)]
    pub reason: Option<String>,
    /// When it was ingested.
    pub created_at: Timestamp,
}
