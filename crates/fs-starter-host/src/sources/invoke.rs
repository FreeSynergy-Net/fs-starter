//! Running one command through the module service (`FormSource.invoke`, Level 1): the contract key, the record and the arguments by the
//! FIELD names of the module go up; a refusal comes back in the ANSWER (`failed` + `problem`) and becomes its kind as a code, a broken
//! line or service (an RPC exception) is `host-notice-data-unreachable`. What the command made or changed (`result`) is given back.
//!
//! Pattern: Command (a value: key, record, arguments) sent over the port; the answer is read as foreign data (clean text, cut).

use std::path::Path;

use fs_forms_run::Coded;
use fs_interfaces::formsource_capnp::form_source;

use super::answer::{Limits, clean};
use super::known::Known;
use super::refusal::kind_code;
use super::service::{ANSWER, UNREACHABLE, cause, finish, run, socket_of, text, up};

/// What a command made or changed, as the service says it (`WriteResult`): the record (empty for a delete) and a short text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written {
    /// The id of the record the command made or changed; empty = none (for example a delete).
    pub record: String,
    /// A short text for people (a code, never a foreign sentence); empty = none.
    pub text: String,
}

/// Run `command` (the contract key) on `record` (empty = none) with `args` (field of the module, value as text) through the service
/// (the check that it is a command of the own name space is made by the caller).
///
/// # Errors
/// The kind of the refusal as a code (`form-source-bad-arguments`, `form-source-forbidden` ...), `host-notice-invoke-failed` for a
/// refusal that has no readable kind, else a code `host-notice-data-*`.
pub fn call(
    known: &Known,
    command: &str,
    record: &str,
    args: &[(String, String)],
    limits: &Limits,
    end: bool,
) -> Result<Written, Coded> {
    run(limits, known, async {
        let socket = socket_of(&known.name)?;
        let started = std::cell::Cell::new(false);
        let got = call_up(known, command, record, args, limits, &socket, &started).await;
        finish(&socket, &started, end).await;
        got
    })
}

async fn call_up(
    known: &Known,
    command: &str,
    record: &str,
    args: &[(String, String)],
    limits: &Limits,
    socket: &Path,
    started: &std::cell::Cell<bool>,
) -> Result<Written, Coded> {
    let socket = socket.to_path_buf();
    let single = up(known, &socket, limits, started).await?;
    let mut request = single.connection.client.invoke_request();
    {
        let mut p = request.get();
        p.set_command(command);
        p.set_record(record);
        let mut list = p.init_args(u32::try_from(args.len()).unwrap_or(u32::MAX));
        for (i, (path, value)) in (0..).zip(args) {
            let mut v = list.reborrow().get(i);
            v.set_path(path);
            v.set_text(value);
        }
    }
    let answer = request
        .send()
        .promise
        .await
        .map_err(|_| cause(UNREACHABLE, &known.name, "invoke"))?;
    let answer = answer
        .get()
        .map_err(|_| cause(ANSWER, &known.name, "invoke"))?;
    read_answer(answer, &known.name, command, limits)
}

/// The answer of `invoke` as the host takes it: a refusal is its kind as a code, a done command is what it wrote.
fn read_answer(
    answer: form_source::invoke_results::Reader<'_>,
    module: &str,
    command: &str,
    limits: &Limits,
) -> Result<Written, Coded> {
    if answer.get_failed() {
        let kind = answer
            .get_problem()
            .and_then(fs_interfaces::common_capnp::problem::Reader::get_type)
            .ok()
            .and_then(|t| t.to_str().ok().and_then(kind_code));
        return Err(cause(
            kind.as_deref().unwrap_or("host-notice-invoke-failed"),
            module,
            "service",
        )
        .with("command", command));
    }
    written(answer.get_result(), limits).map_err(|()| cause(ANSWER, module, "result"))
}

fn written(
    result: capnp::Result<fs_interfaces::formsource_capnp::write_result::Reader<'_>>,
    limits: &Limits,
) -> Result<Written, ()> {
    let result = result.map_err(|_| ())?;
    Ok(Written {
        record: text(result.get_record(), 256)?,
        text: clean(
            result
                .get_text()
                .map_err(|_| ())?
                .to_str()
                .map_err(|_| ())?,
            limits.text,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An answer of `invoke`: `failed`, the type of the problem, the record and the text of the result.
    fn answer(
        failed: bool,
        problem_type: &str,
        record: &str,
        text: &str,
    ) -> capnp::message::Builder<capnp::message::HeapAllocator> {
        let mut msg = capnp::message::Builder::new_default();
        let mut r = msg.init_root::<form_source::invoke_results::Builder>();
        r.set_failed(failed);
        r.reborrow().init_problem().set_type(problem_type);
        let mut w = r.init_result();
        w.set_record(record);
        w.set_text(text);
        msg
    }

    fn read(
        msg: &capnp::message::Builder<capnp::message::HeapAllocator>,
    ) -> Result<Written, Coded> {
        let r = msg
            .get_root_as_reader::<form_source::invoke_results::Reader>()
            .unwrap();
        read_answer(r, "planning", "card.move", &Limits::default())
    }

    #[test]
    fn a_refusal_gives_its_kind_as_the_code_and_a_kind_nobody_can_read_gives_failed() {
        let c = read(&answer(true, "form-source-bad-arguments", "", "")).unwrap_err();
        assert_eq!(c.code, "form-source-bad-arguments");
        assert!(c.params.contains(&("command".into(), "card.move".into())));
        let c = read(&answer(true, "https://x.example/evil", "", "")).unwrap_err();
        assert_eq!(c.code, "host-notice-invoke-failed");
    }

    #[test]
    fn a_done_command_gives_the_record_it_made_with_clean_text() {
        let got = read(&answer(false, "", "card-7", "ok\u{1b}[31m")).unwrap();
        assert_eq!(got.record, "card-7");
        assert_eq!(got.text, "ok\u{FFFD}[31m");
        assert_eq!(
            read(&answer(false, "", "", "")).unwrap(),
            Written::default()
        );
    }
}
