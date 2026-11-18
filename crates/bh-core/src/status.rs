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

/// A parsed status-log line (`verb: message`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusLine {
    /// The verb that leads the line.
    pub verb: StatusVerb,
    /// Everything after the first colon, trimmed.
    pub message: String,
}

impl StatusLine {
    /// Parses one `verb: message` line.
    ///
    /// Leading and trailing whitespace is ignored, and the split happens at
    /// the first colon so the message may itself contain colons. The verb is
    /// matched case-sensitively against the protocol tokens; anything else
    /// becomes [`StatusVerb::Unknown`].
    pub fn parse(line: &str) -> Result<Self, StatusParseError> {
        let line = line.trim();
        let (token, message) = line
            .split_once(':')
            .ok_or(StatusParseError::MissingSeparator)?;
        let token = token.trim();
        if token.is_empty() {
            return Err(StatusParseError::EmptyVerb);
        }
        let verb = match token {
            "working" => StatusVerb::Working,
            "needs-decision" => StatusVerb::NeedsDecision,
            "blocked" => StatusVerb::Blocked,
            "pr" => StatusVerb::Pr,
            "done" => StatusVerb::Done,
            other => StatusVerb::Unknown(other.to_owned()),
        };
        Ok(Self {
            verb,
            message: message.trim().to_owned(),
        })
    }
}

/// Reasons a status line cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StatusParseError {
    /// The line has no `:` separator.
    #[error("status line has no `:` separator")]
    MissingSeparator,
    /// The line has a separator but no verb before it.
    #[error("status line has an empty verb")]
    EmptyVerb,
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

    #[test]
    fn parses_known_and_unknown_verbs() {
        let line = StatusLine::parse("  needs-decision: pick a format  ").unwrap();
        assert_eq!(line.verb, StatusVerb::NeedsDecision);
        assert_eq!(line.message, "pick a format");

        let line = StatusLine::parse("deploying: new verb").unwrap();
        assert_eq!(line.verb, StatusVerb::Unknown("deploying".to_owned()));
    }

    #[test]
    fn keeps_colons_inside_the_message() {
        let line = StatusLine::parse("blocked: waiting on X: then Y").unwrap();
        assert_eq!(line.verb, StatusVerb::Blocked);
        assert_eq!(line.message, "waiting on X: then Y");
    }

    #[test]
    fn rejects_separatorless_and_verbless_lines() {
        assert_eq!(
            StatusLine::parse("working"),
            Err(StatusParseError::MissingSeparator)
        );
        assert_eq!(
            StatusLine::parse(": no verb"),
            Err(StatusParseError::EmptyVerb)
        );
    }
}
