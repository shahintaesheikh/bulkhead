//! `bh` library surface.
//!
//! The binary is a thin wrapper over [`cli::run`]. Keeping the logic in a
//! library lets integration tests drive `init` and `doctor` against temporary
//! homes without spawning the CLI.

pub mod agent_folder;
pub mod attach;
pub mod cli;
pub mod config;
pub mod delivery;
pub mod doctor;
pub mod fsx;
pub mod ghaxi;
pub mod herdr;
pub mod home;
pub mod init;
pub mod no_mistakes;
pub mod orchestrator;
pub mod paths;
pub mod registry;
pub mod repo;
pub mod task;
pub mod treehouse;
