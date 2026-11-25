//! The pages of the forms of one module: one `page` call per read ([`Wire`]: the entry and the scope of its source), the answer turned
//! into rows by channel. Everything the service answers is foreign data: [`super::answer`] holds the borders.
//!
//! Pattern: Adapter (records by field of the module in, rows by channel out) over `FormSource.page`.

use fs_forms_bind::ModuleForm;
use fs_forms_run::{Coded, FormData, FormRow};
use fs_interfaces::formsource_capnp::form_source;

use super::answer::Limits;
use super::entries::Wire;
use super::range::Version;
use super::refusal::refusal_or;
use super::service::{ANSWER, UNREACHABLE, cause, text};

#[cfg(test)]
#[path = "rows_tests.rs"]
mod rows_tests;

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

/// What [`read_pages`] read: the forms, and the sources (from number 2) it could not read, as `(source number, code)`.
#[derive(Debug, Default)]
pub struct Pages {
    /// The rows of every read that worked, in the order of the wires.
    pub forms: Vec<FormData>,
    /// A source of the layout that gave an error: the window runs without it and says why (`FormRecords.problems`).
    pub problems: Vec<(u32, Coded)>,
}

/// The pages of `wires` on `client`, in the order of `wires`; the scope of each page is the scope of its wire (never a word of the
/// form name). The answer is refused (`Err`) when it is over a border. A wire of source 2 or higher that the service refuses (scope
/// unknown, board gone ...) is NOT a failure of the whole read: it becomes a problem of that source and the others are read (the port
/// `Sources` says so). A wire of the first source (or without a layout, source 0) stays an error: without it there is nothing to draw.
///
/// # Errors
/// `host-notice-data-answer` (cause `page`, `limit`, `may`) or the refusal of the service as its code, for the first source.
pub async fn read_pages(
    client: &form_source::Client,
    module: &str,
    wires: &[Wire],
    limits: &Limits,
) -> Result<Pages, Coded> {
    let mut bytes = 0;
    let mut out = Pages::default();
    for wire in wires {
        match read_one(client, module, wire, limits, &mut bytes).await {
            Ok(data) => out.forms.push(data),
            Err(code) if wire.source >= 2 => out.problems.push((wire.source, code)),
            Err(code) => return Err(code),
        }
    }
    Ok(out)
}

async fn read_one(
    client: &form_source::Client,
    module: &str,
    wire: &Wire,
    limits: &Limits,
    bytes: &mut usize,
) -> Result<FormData, Coded> {
    let form = &wire.form;
    let mut request = client.page_request();
    {
        let mut p = request.get();
        // ONE spelling on the wire: `name@x.y.z` (a form never raises its patch)
        p.set_form(fs_interfaces::forms::key(
            &form.form.name,
            &Version::new(u64::from(form.form.major), u64::from(form.form.minor), 0),
        ));
        p.set_scope(wire.scope.as_str());
        p.set_limit(limits.rows);
    }
    let answer = request
        .send()
        .promise
        .await
        .map_err(|e| refusal_or(&e, module, "page", UNREACHABLE))?;
    let page = answer
        .get()
        .and_then(form_source::page_results::Reader::get_page)
        .map_err(|_| cause(ANSWER, module, "page"))?;
    let list = page
        .get_records()
        .map_err(|_| cause(ANSWER, module, "page"))?;
    let rows = rows(list, form, limits, bytes).map_err(|()| cause(ANSWER, module, "limit"))?;
    let mut data = FormData::read_only(form.clone(), rows);
    // the rows say which source of the layout they are for: the lanes of a grouping are found by this number and type
    data.source = wire.source;
    data.data_type.clone_from(&wire.data_type);
    data.may = may(page.get_may(), limits).map_err(|()| cause(ANSWER, module, "may"))?;
    Ok(data)
}
