//! Opaque identifiers.
//!
//! Every id is a distinct newtype, so a `TaskId` can never be passed where a
//! `PaneId` is expected. The inner string is whatever the owning system uses:
//! a repo slug for `RepoId`, a generated token for `TaskId` and `WakeId`, and
//! herdr's public handle (`w1`, `w1:t1`, `w1:p1`) for workspace, tab and pane.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Defines a thin, `Display`able newtype around a `String`.
///
/// This is deliberately a declarative macro: the alternative is eight
/// byte-identical structs, which obscures the fact that they are the same
/// concept applied to different ids.
macro_rules! string_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wraps a raw identifier.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns the raw identifier.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }
    };
}

string_id!(
    /// Identifier for a managed repository: a filesystem-safe slug.
    RepoId
);
string_id!(
    /// Identifier for one crew task (`<repo-id>/tasks/<task-id>/`).
    TaskId
);
string_id!(
    /// Identifier for one durable wake. Stable across redelivery.
    WakeId
);
string_id!(
    /// herdr pane handle (`w1:p1`).
    PaneId
);
string_id!(
    /// herdr tab handle (`w1:t1`).
    TabId
);
string_id!(
    /// herdr workspace handle (`w1`).
    WorkspaceId
);
string_id!(
    /// Identifier for one plan under `<repo-id>/plans/`.
    PlanId
);
string_id!(
    /// Pi session reference: the value passed to `pi --session <ref>`.
    SessionRef
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newtype_display_and_conversions_agree() {
        let repo = RepoId::from("bulkhead");
        assert_eq!(repo.as_str(), "bulkhead");
        assert_eq!(repo.to_string(), "bulkhead");
        assert_eq!(RepoId::new(String::from("bulkhead")), repo);
    }
}
