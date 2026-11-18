//! The crew status protocol verbs.
//!
//! Crewmates append lines of the form `verb: message` to their task's
//! `status.log`. The parser (a later unit) turns a line into a [`StatusVerb`]
//! plus its message.

use serde::{Deserialize, Serialize};

/// The verb at the start of a status-log line (`working: ...`).
///
/// Unknown verbs are preserved verbatim in [`StatusVerb::Unknown`] rather than
/// dropped or panicking, so a newer crew build cannot crash an older daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StatusVerb {
    /// The crewmate is making progress.
    Working,
    /// The crewmate needs a decision before it can continue.
    NeedsDecision,
    /// The crewmate is blocked by something outside its control.
    Blocked,
    /// The crewmate opened or updated a PR.
    Pr,
    /// The crewmate finished.
    Done,
    /// A verb this build does not know, kept verbatim.
    Unknown(String),
}

impl StatusVerb {
    /// The protocol token, without the trailing colon.
    pub fn as_token(&self) -> &str {
        match self {
            Self::Working => "working",
            Self::NeedsDecision => "needs-decision",
            Self::Blocked => "blocked",
            Self::Pr => "pr",
            Self::Done => "done",
            Self::Unknown(raw) => raw,
        }
    }

    /// Whether this verb ends a task.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_match_the_protocol() {
        assert_eq!(StatusVerb::Working.as_token(), "working");
        assert_eq!(StatusVerb::NeedsDecision.as_token(), "needs-decision");
        assert_eq!(StatusVerb::Blocked.as_token(), "blocked");
        assert_eq!(StatusVerb::Pr.as_token(), "pr");
        assert_eq!(StatusVerb::Done.as_token(), "done");
        assert_eq!(
            StatusVerb::Unknown("waiting".to_owned()).as_token(),
            "waiting"
        );
    }

    #[test]
    fn only_done_is_terminal() {
        assert!(StatusVerb::Done.is_terminal());
        assert!(!StatusVerb::Blocked.is_terminal());
    }
}
