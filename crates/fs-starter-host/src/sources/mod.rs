//! The data sources of the host: the adapter `Sources` of `fs-forms-run` for the modules of the starter (see [`starter`]).
//!
//! Pattern: Facade (declarations and re-exports only).

pub mod answer;
pub mod declared;
mod known;
pub mod range;
mod service;
mod starter;

pub use answer::Limits;
pub use known::{Known, Modules};
pub use starter::{StarterSources, bare, line};
