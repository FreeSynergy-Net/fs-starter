//! The receiver of the changes of ONE subscription: tells each change to the [`Watch`], and hands its end (`ended`) to it with the
//! status of the problem.
//!
//! Pattern: Observer (the `ChangeSink` of the module is the observer; the weak link keeps a dropped window from being held alive).

use std::rc::{Rc, Weak};

use fs_interfaces::formsource_capnp::change_sink;

use super::reply::change_of;
use super::watch_resume::{ENDED, Watch};

/// The receiver of the changes of one subscription (the `ChangeSink` of the module).
pub(super) struct Sink {
    pub(super) watch: Weak<Watch>,
    pub(super) generation: u64,
}

impl change_sink::Server for Sink {
    #[allow(clippy::unused_async_trait_impl)] // the generated trait wants an async method
    async fn changed(self: Rc<Self>, p: change_sink::ChangedParams) -> Result<(), capnp::Error> {
        let change = change_of(p.get()?.get_change()?)?;
        if let Some(watch) = self.watch.upgrade() {
            watch.take(&change, "");
        }
        Ok(())
    }

    /// Never answers with an error (contract): the problem is read, the rest is the resume.
    #[allow(clippy::unused_async_trait_impl)]
    async fn ended(
        self: Rc<Self>,
        p: change_sink::EndedParams,
        _: change_sink::EndedResults,
    ) -> Result<(), capnp::Error> {
        let status = p
            .get()
            .and_then(change_sink::ended_params::Reader::get_problem)
            .map_or(
                ENDED,
                fs_interfaces::common_capnp::problem::Reader::get_status,
            );
        if let Some(watch) = self.watch.upgrade() {
            watch.ended(self.generation, status);
        }
        Ok(())
    }
}
