//! `--sample-data`: every `[[source]]` of the description is served by a `SampleModule` made from the description itself
//! (`fs-forms-testdata`: the forms the blocks draw, the wishes of `[sample]`, else the defaults of the declaration), held IN THIS
//! PROCESS behind the wrapper `Guarded` and read, held and written through the same capability a socket module has: `read_pages`,
//! the `Session` (history, `app.undo`), the port `Writer`. Nothing leaves the process: there is no socket, no service, no store.
//! The provider says `fake` ([`SAMPLE_PROVIDER`]), so the window shows "Sample data" (`FormRecords::is_sample`).
//!
//! Only `Sources::connect_sample` comes here, and `fs-forms-run` calls it only for the word `--sample-data`.
//!
//! Pattern: Adapter (a sample module in place of a service behind the one road of the host: read, hold, session, writer) + Facade.

mod bind;
mod make;
mod unit;
mod writer;

pub(in crate::sources) use make::connect;
