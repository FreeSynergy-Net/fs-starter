//! The data sources of the host: the adapter `Sources` of `fs-forms-run` for the modules of the starter (see [`starter`]).
//!
//! Pattern: Facade (declarations and re-exports only).

pub mod answer;
mod connect;
pub mod declared;
mod end;
pub mod entries;
mod held;
mod invoke;
mod known;
mod level;
pub mod pages;
pub mod range;
mod refusal;
mod reply;
mod sample;
mod service;
mod service_keep;
mod session;
mod starter;
mod watch;
mod watch_resume;
#[cfg(all(test, unix))]
mod watch_resume_fake;
mod watch_seen;
mod watch_sink;
mod writer;

pub use answer::Limits;
pub use declared::bare;
pub use held::HeldRecords;
pub use invoke::Written;
pub use known::{Known, Modules};
pub use starter::{StarterSources, line};
