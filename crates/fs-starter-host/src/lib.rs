//! `fs-starter-host`: the host. It opens the window of a program from its description (`--description PATH` or a bare PATH: a
//! blueprint folder with `scaffold.toml`, a folder with `forms/*.toml`, or one file), without building the program. It has no
//! program name and no special case; every word it does not know is handed to `fs-forms-run`, which owns the window.
//!
//! The single instance is built, per program (`instance.rs`, ADR 0097): a second start hands its words to the running window.
//! `--new-instance` opens a further window on its own endpoint (`slots.rs`).
//! The MCP switch is built (`McpOff`, see `shell.rs`: MCP is off) and the start hooks run (a no-op until `fs-update` is bound). What the
//! starter shell of this crate's sibling `fs-starter` will add (session, update, help, MCP entry) is
//! NOT built here: it is named once on stderr with the code `skeleton-part-pending`.
//!
//! Pattern: Facade: this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod check_sources;
mod check_system;
mod host;
pub mod instance;
pub mod open_setting;
mod package;
mod pending;
mod program_arg;
pub mod registry;
mod shell;
pub mod slots;
pub mod sources;
pub mod update;

pub use host::{HELP, run, version_line};
pub use package::{Package, line as package_line, read as read_package};
pub use pending::{CODE, PENDING, note};
pub use shell::NoopHook;
pub use update::ACTIVE_CODE;
