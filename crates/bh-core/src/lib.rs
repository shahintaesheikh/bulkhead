//! bulkhead's pure domain layer.
//!
//! `bh-core` holds types and decision functions only. It performs no I/O, no
//! async work and no process spawning (architecture invariant 1), which keeps
//! every supervision rule unit-testable without a runtime.
//!
//! Binaries wire I/O to these types; they do not reimplement them.

pub mod brief;
pub mod classify;
pub mod config;
pub mod event;
pub mod frontmatter;
pub mod ids;
pub mod pane;
pub mod plan;
pub mod protocol;
pub mod status;
pub mod task;
pub mod time;
pub mod wake;

pub use brief::{BriefContext, render as render_brief};
pub use classify::{RepoState, SupervisionState, TaskState, check_stalls, classify};
pub use config::{AwayPolicy, DeliveryMode, Registry, RegistryEntry, RepoConfig};
pub use event::Event;
pub use ids::{PaneId, PlanId, RepoId, SessionRef, TabId, TaskId, WakeId, WorkspaceId};
pub use pane::PaneState;
pub use plan::{Plan, TriageState};
pub use protocol::{Envelope, ProtocolError, ProtocolMessage, ProtocolVersion};
pub use status::{StatusLine, StatusParseError, StatusVerb};
pub use task::Task;
pub use time::{Seconds, Timestamp};
pub use wake::{Wake, WakeKind, WakeRecord};
