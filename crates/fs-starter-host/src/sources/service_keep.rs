//! The read of a held one-time run: like `fetch_level`, but the service it started is NOT ended (the guard [`super::end`] does it).
//!
//! Pattern: Facade over `read_all` of [`super::service`] (the same read, one flag more).

use fs_forms_bind::ModuleForm;
use fs_forms_run::{Coded, FormRecords};

use super::answer::Limits;
use super::known::Known;
use super::level::Levels;
use super::service::{UNREACHABLE, cause, read_all};

/// [`fetch_level`] that never ends the service: the third part says whether THIS read started it (the caller then owns its end).
/// Even a failed read says it: the service it started is still up.
///
/// # Errors
/// As [`fetch_level`]; the flag is given with the error too.
pub fn fetch_keeping(
    known: &Known,
    forms: &[ModuleForm],
    limits: &Limits,
) -> (Result<(FormRecords, Levels), Coded>, bool) {
    let none = |e| (Err(e), false);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| cause(UNREACHABLE, &known.name, "runtime"));
    let runtime = match runtime {
        Ok(r) => r,
        Err(e) => return none(e),
    };
    let local = tokio::task::LocalSet::new();
    local
        .block_on(&runtime, async {
            tokio::time::timeout(limits.time, read_all(known, forms, limits, false)).await
        })
        .unwrap_or_else(|_| (Err(cause(UNREACHABLE, &known.name, "time")), false))
}
