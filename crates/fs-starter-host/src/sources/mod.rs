//! The data sources of the host: the adapter `Sources` of `fs-forms-run` for the modules of the starter (see [`starter`]).
//!
//! Pattern: Facade (declarations and re-exports only).

pub mod answer;
mod connect;
pub mod declared;
mod held;
mod invoke;
mod known;
pub mod range;
mod refusal;
mod service;
mod starter;
mod writer;

pub use answer::Limits;
pub use held::HeldRecords;
pub use invoke::Written;
pub use known::{Known, Modules};
pub use starter::{StarterSources, bare, line};
