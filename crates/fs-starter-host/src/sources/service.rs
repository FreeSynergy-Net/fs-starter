//! Talking to the module service: find the program in the module folder, make sure ONE service runs (`ensure_single`, the version
//! handshake of fs-rpc, the heartbeat), say hello (`FormSource` level), read the pages of every form the module declares, or run one
//! command. The socket is the one `default_socket(<module>)` gives in the runtime folder of this person, nothing else (border 4).
//! Everything the service answers is foreign data: [`super::answer`] holds the borders.
//!
//! Pattern: Facade over fs-rpc (`ensure_single`, `connect`) + Adapter (records by field of the module in, rows by channel out).

use std::path::{Path, PathBuf};

use fs_forms_bind::ModuleForm;
use fs_forms_run::{Coded, FormData, FormRecords, FormRow};
use fs_interfaces::forms::FORM_SOURCE_MIN_LEVEL;
use fs_interfaces::formsource_capnp::form_source;
use fs_rpc::{CommandStarter, Expected, ServerStarter, SingleTimes, ensure_single, stop_server};

use super::answer::{Limits, clean};
use super::known::Known;
use super::level::{self, Levels};
use super::range::Version;
use super::refusal::refusal_or;

#[cfg(test)]
#[path = "rows_tests.rs"]
mod rows_tests;

pub(super) const UNREACHABLE: &str = "host-notice-data-unreachable";
const CONTRACT: &str = "host-notice-data-contract";
pub(super) const ANSWER: &str = "host-notice-data-answer";

pub(super) fn cause(code: &str, module: &str, why: &str) -> Coded {
    Coded::new(code).with("module", module).with("cause", why)
}

/// The socket of the module: only `default_socket(<module>)`, never a path from a description, a word or the service.
pub(super) fn socket_of(module: &str) -> Result<PathBuf, Coded> {
    fs_rpc::default_socket(module).ok_or_else(|| Coded::new("host-notice-data-no-socket"))
}

/// The starter that notes that THIS call started the service (a service that was up already is not ours to end).
struct Noted<'a> {
    inner: CommandStarter,
    started: &'a std::cell::Cell<bool>,
}

impl ServerStarter for Noted<'_> {
    fn start(&self) -> std::io::Result<()> {
        self.started.set(true);
        self.inner.start()
    }
}

/// How the service of a module is started and what it must say about itself: the ONE place (border 3: only the program file of the
/// module folder; any version, the contract version was compared with the catalog of the module by the host, border 2, here only the
/// protocol). `up` and the hold of the window both take it from here.
pub(super) fn parts(known: &Known) -> Result<(PathBuf, Expected, CommandStarter), Coded> {
    let exe = known
        .service()
        .map_err(|why| cause(UNREACHABLE, &known.name, why))?;
    let starter = CommandStarter::new(&exe, ["serve"]);
    Ok((exe, Expected::new(""), starter))
}

/// A service that is up (started from the folder of the module if none runs; `ours` is set when THIS call started it) and the
/// capability of `FormSource` on it.
pub(super) async fn up(
    known: &Known,
    socket: &Path,
    limits: &Limits,
    ours: &std::cell::Cell<bool>,
) -> Result<fs_rpc::Single<form_source::Client>, Coded> {
    let (exe, expected, inner) = parts(known)?;
    let starter = Noted {
        inner,
        started: ours,
    };
    let times = SingleTimes {
        overall: limits.time,
        ..SingleTimes::default()
    };
    ensure_single::<form_source::Client>(socket, &expected, &starter, &times, Some(&exe))
        .await
        .map_err(|_| cause(UNREACHABLE, &known.name, "start"))
}

/// The heartbeat is a duty of every module service (ADR 0104): a service that does not answer it is not trusted with the window.
#[cfg(unix)]
async fn beat(known: &Known, socket: &Path) -> Result<(), Coded> {
    fs_rpc::ask_beat(socket, std::time::Duration::from_secs(2))
        .await
        .map(|_| ())
        .map_err(|_| cause(UNREACHABLE, &known.name, "heartbeat"))
}

#[cfg(not(unix))]
#[allow(clippy::unused_async)] // same signature as the unix one (the caller awaits it); the heartbeat of fs-rpc is unix only, so nothing to await here
async fn beat(_: &Known, _: &Path) -> Result<(), Coded> {
    Ok(())
}

pub(super) fn text(t: capnp::Result<capnp::text::Reader<'_>>, max: usize) -> Result<String, ()> {
    Ok(clean(t.map_err(|_| ())?.to_str().map_err(|_| ())?, max))
}

/// The commands a person may run (`may` of a record or of the page), as clean text; at most `limits.values` of them.
fn may(
    list: capnp::Result<capnp::text_list::Reader<'_>>,
    limits: &Limits,
) -> Result<Vec<String>, ()> {
    let list = list.map_err(|_| ())?;
    (0..list
        .len()
        .min(u32::try_from(limits.values).unwrap_or(u32::MAX)))
        .map(|i| text(list.get(i), limits.text))
        .collect()
}

/// One page of one form as rows by channel; the answer is refused (`Err`) when it is over a border.
fn rows(
    list: capnp::struct_list::Reader<'_, fs_interfaces::formsource_capnp::record_view::Owned>,
    form: &ModuleForm,
    limits: &Limits,
    bytes: &mut usize,
) -> Result<Vec<FormRow>, ()> {
    if list.len() > limits.rows {
        return Err(());
    }
    let mut out = Vec::new();
    for record in list {
        let key = text(record.get_id(), 256)?;
        let values = record.get_values().map_err(|_| ())?;
        let mut by_channel = Vec::new();
        for (channel, fields) in &form.map {
            let mut got: Vec<String> = Vec::new();
            for field in fields {
                for v in values {
                    if v.get_path().map_err(|_| ())?.to_str().map_err(|_| ())? == field {
                        got.push(text(v.get_text(), limits.text)?);
                    }
                }
            }
            got.truncate(limits.values);
            *bytes += got.iter().map(String::len).sum::<usize>();
            if !got.is_empty() {
                by_channel.push((channel.clone(), got));
            }
        }
        *bytes += key.len();
        if *bytes > limits.bytes {
            return Err(());
        }
        out.push(FormRow {
            key,
            values: by_channel,
            // no statement = nothing may be written (the controls stay locked)
            may: may(record.get_may(), limits)?,
        });
    }
    Ok(out)
}

/// A one-time command (`--picture`, `--check-system`) ends the service it started itself: the service has no other user, and the
/// deadline without a user (SB-3) is not there yet. A service that was up before is never ended.
pub(super) async fn finish(socket: &Path, started: &std::cell::Cell<bool>, end: bool) {
    if end && started.get() {
        let _ = stop_server(socket).await;
    }
}

async fn read_all(
    known: &Known,
    forms: &[ModuleForm],
    limits: &Limits,
    end: bool,
) -> Result<(FormRecords, Levels), Coded> {
    let socket = socket_of(&known.name)?;
    let started = std::cell::Cell::new(false);
    let got = read_up(known, forms, limits, &socket, &started).await;
    finish(&socket, &started, end).await;
    got
}

async fn read_up(
    known: &Known,
    forms: &[ModuleForm],
    limits: &Limits,
    socket: &Path,
    started: &std::cell::Cell<bool>,
) -> Result<(FormRecords, Levels), Coded> {
    let socket = socket.to_path_buf();
    let single = up(known, &socket, limits, started).await?;
    beat(known, &socket).await?;
    let client = single.connection.client.clone();
    let hello = client
        .hello_request()
        .send()
        .promise
        .await
        .map_err(|_| cause(UNREACHABLE, &known.name, "hello"))?;
    let hello = hello
        .get()
        .map_err(|_| cause(ANSWER, &known.name, "hello"))?;
    // the gate is cumulative: every level >= the minimum is taken, the host uses min(level, its own) (`level`); below it: refused with the reason
    let said = hello.get_level();
    let Some(used) = level::negotiate(said) else {
        return Err(cause(CONTRACT, &known.name, "level")
            .with("level", said.to_string())
            .with("minimum", FORM_SOURCE_MIN_LEVEL.to_string()));
    };
    let provider =
        text(hello.get_provider(), 64).map_err(|()| cause(ANSWER, &known.name, "provider"))?;
    let mut bytes = 0;
    let mut out = Vec::new();
    for form in forms {
        let mut request = client.page_request();
        {
            let mut p = request.get();
            // ONE spelling on the wire: `name@x.y.z` (a form never raises its patch)
            p.set_form(fs_interfaces::forms::key(
                &form.form.name,
                &Version::new(u64::from(form.form.major), u64::from(form.form.minor), 0),
            ));
            p.set_scope("");
            p.set_limit(limits.rows);
        }
        let answer = request
            .send()
            .promise
            .await
            .map_err(|e| refusal_or(&e, &known.name, "page", UNREACHABLE))?;
        let page = answer
            .get()
            .and_then(form_source::page_results::Reader::get_page)
            .map_err(|_| cause(ANSWER, &known.name, "page"))?;
        let list = page
            .get_records()
            .map_err(|_| cause(ANSWER, &known.name, "page"))?;
        let rows = rows(list, form, limits, &mut bytes)
            .map_err(|()| cause(ANSWER, &known.name, "limit"))?;
        let mut data = FormData::read_only(form.clone(), rows);
        data.may = may(page.get_may(), limits).map_err(|()| cause(ANSWER, &known.name, "may"))?;
        out.push(data);
    }
    Ok((
        FormRecords {
            provider,
            forms: out,
            ..FormRecords::default()
        },
        Levels {
            service: said,
            used,
        },
    ))
}

pub(super) fn run<T>(
    limits: &Limits,
    known: &Known,
    work: impl std::future::Future<Output = Result<T, Coded>>,
) -> Result<T, Coded> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| cause(UNREACHABLE, &known.name, "runtime"))?;
    let local = tokio::task::LocalSet::new();
    local
        .block_on(&runtime, async {
            tokio::time::timeout(limits.time, work).await
        })
        .unwrap_or_else(|_| Err(cause(UNREACHABLE, &known.name, "time")))
}

/// Start (if needed), check and read: the records of every declared form, as rows by channel, and the levels (what the service said and
/// what the host works with: `min(level, its own)`; a service below the minimum is refused, cause `level`, with `level` and `minimum`).
///
/// # Errors
/// A code `host-notice-data-*` with the module and the cause; nothing half read is returned. `end`: end the service if this call
/// started it (one-time commands).
pub fn fetch_level(
    known: &Known,
    forms: &[ModuleForm],
    limits: &Limits,
    end: bool,
) -> Result<(FormRecords, Levels), Coded> {
    run(limits, known, read_all(known, forms, limits, end))
}
