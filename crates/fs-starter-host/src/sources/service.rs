//! Talking to the module service: find the program in the module folder, make sure ONE service runs (`ensure_single`, the version
//! handshake of fs-rpc, the heartbeat), say hello (`FormSource` level), read the pages of every form the module declares, or run one
//! command. The socket is the one `default_socket(<module>)` gives in the runtime folder of this person, nothing else (border 4).
//! Everything the service answers is foreign data: [`super::answer`] holds the borders.
//!
//! Pattern: Facade over fs-rpc (`ensure_single`, `connect`) + Adapter (records by field of the module in, rows by channel out).

use std::path::{Path, PathBuf};

use fs_forms_bind::ModuleForm;
use fs_forms_run::{Coded, FormData, FormRecords, FormRow};
use fs_interfaces::forms::FORM_SOURCE_LEVEL;
use fs_interfaces::formsource_capnp::form_source;
use fs_rpc::{CommandStarter, Expected, SingleTimes, ensure_single};

use super::answer::{Limits, clean};
use super::known::Known;

#[cfg(test)]
#[path = "rows_tests.rs"]
mod rows_tests;

const UNREACHABLE: &str = "host-notice-data-unreachable";
const CONTRACT: &str = "host-notice-data-contract";
const ANSWER: &str = "host-notice-data-answer";

fn cause(code: &str, module: &str, why: &str) -> Coded {
    Coded::new(code).with("module", module).with("cause", why)
}

/// The socket of the module: only `default_socket(<module>)`, never a path from a description, a word or the service.
fn socket_of(module: &str) -> Result<PathBuf, Coded> {
    fs_rpc::default_socket(module).ok_or_else(|| Coded::new("host-notice-data-no-socket"))
}

/// A service that is up (started from the folder of the module if none runs) and the capability of `FormSource` on it.
async fn up(
    known: &Known,
    socket: &Path,
    limits: &Limits,
) -> Result<fs_rpc::Single<form_source::Client>, Coded> {
    let exe = known
        .service()
        .map_err(|why| cause(UNREACHABLE, &known.name, why))?;
    // any version: the contract version was compared with the catalog of the module by the host (border 2), here only the protocol
    let expected = Expected::new("");
    let starter = CommandStarter::new(&exe, ["serve"]);
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
async fn beat(_: &Known, _: &Path) -> Result<(), Coded> {
    Ok(())
}

fn text(t: capnp::Result<capnp::text::Reader<'_>>, max: usize) -> Result<String, ()> {
    Ok(clean(t.map_err(|_| ())?.to_str().map_err(|_| ())?, max))
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
        });
    }
    Ok(out)
}

async fn read_all(
    known: &Known,
    forms: &[ModuleForm],
    limits: &Limits,
) -> Result<FormRecords, Coded> {
    let socket = socket_of(&known.name)?;
    let single = up(known, &socket, limits).await?;
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
    if hello.get_level() != FORM_SOURCE_LEVEL {
        return Err(cause(CONTRACT, &known.name, "level"));
    }
    let provider =
        text(hello.get_provider(), 64).map_err(|()| cause(ANSWER, &known.name, "provider"))?;
    let mut bytes = 0;
    let mut out = Vec::new();
    for form in forms {
        let mut request = client.page_request();
        {
            let mut p = request.get();
            p.set_form(format!(
                "{}@{}.{}",
                form.form.name, form.form.major, form.form.minor
            ));
            p.set_scope("");
            p.set_limit(limits.rows);
        }
        let answer = request
            .send()
            .promise
            .await
            .map_err(|_| cause(UNREACHABLE, &known.name, "page"))?;
        let page = answer
            .get()
            .and_then(form_source::page_results::Reader::get_page)
            .and_then(fs_interfaces::formsource_capnp::record_page::Reader::get_records)
            .map_err(|_| cause(ANSWER, &known.name, "page"))?;
        let rows = rows(page, form, limits, &mut bytes)
            .map_err(|()| cause(ANSWER, &known.name, "limit"))?;
        out.push(FormData {
            declared: form.clone(),
            rows,
        });
    }
    Ok(FormRecords {
        provider,
        forms: out,
    })
}

fn run<T>(
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

/// Start (if needed), check and read: the records of every declared form, as rows by channel.
///
/// # Errors
/// A code `host-notice-data-*` with the module and the cause; nothing half read is returned.
pub fn fetch(known: &Known, forms: &[ModuleForm], limits: &Limits) -> Result<FormRecords, Coded> {
    run(limits, known, read_all(known, forms, limits))
}

/// Run `command` on `record` through the service (the check that it is a command of the own name space is made by the caller).
///
/// # Errors
/// A code `host-notice-data-*`, or `host-notice-invoke-failed` when the service says it did not do it.
pub fn call(known: &Known, command: &str, record: &str, limits: &Limits) -> Result<(), Coded> {
    run(limits, known, async {
        let socket = socket_of(&known.name)?;
        let single = up(known, &socket, limits).await?;
        let mut request = single.connection.client.invoke_request();
        {
            let mut p = request.get();
            p.set_command(command);
            p.set_record(record);
        }
        let answer = request
            .send()
            .promise
            .await
            .map_err(|_| cause(UNREACHABLE, &known.name, "invoke"))?;
        let failed = answer
            .get()
            .map_err(|_| cause(ANSWER, &known.name, "invoke"))?
            .get_failed();
        if failed {
            return Err(
                cause("host-notice-invoke-failed", &known.name, "service").with("command", command)
            );
        }
        Ok(())
    })
}
