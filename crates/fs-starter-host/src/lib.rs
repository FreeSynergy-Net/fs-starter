//! `fs-starter-host`: the host. It opens the window of a program from its description (`--description PATH` or a bare PATH: a
//! blueprint folder with `scaffold.toml`, a folder with `forms/*.toml`, or one file), without building the program. It has no
//! program name and no special case; every word it does not know is handed to `fs-forms-run`, which owns the window.
//!
//! What the starter shell of this crate's sibling `fs-starter` will add (single instance, session, update, help, MCP switch) is
//! NOT built here: it is named once on stderr with the code `skeleton-part-pending`.
//!
//! Pattern: Facade: this file only declares modules and re-exports the public API.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod host;
mod package;
mod pending;

pub use host::{HELP, run, version_line};
pub use package::{Package, line as package_line, read as read_package};
pub use pending::{CODE, PENDING, note};
