//! The form entries a module declares (`catalog/forms.json`) as a registry by the key (form, type), and the choice of what a window
//! reads: a layout source says `(number, type, scope)` (`Sources::connect_wanted` of fs-forms-run hands them over, numbered from 1 like
//! `source` of a block), and ONLY the entries of that type are read, each with the scope of its source. A form
//! name alone is not a key: planning declares `rows@1.0.0` twice (type `Lane` and type `Task`) and the scope decides which rows come.
//!
//! Pattern: Registry – Formeinträge nach Schlüssel (Form, Typ)

use fs_forms_bind::ModuleForm;

/// One entry of `forms.json`: the form the module keeps and the data type of its records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The form, its version and its map.
    pub form: ModuleForm,
    /// The data type of the records (`type` of `forms.json`; empty where the module says none).
    pub data_type: String,
}

/// What one layout source asks for: its number, the data type and the scope word that goes to the module as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    /// The number of the source in the layout (`Want::n` of fs-forms-run: counts from 1, like `source` of a block and a binding).
    pub n: u32,
    /// The data type of the source (`type` of the `[[source]]`); empty = the layout does not say (a view made by hand).
    pub data_type: String,
    /// The scope on the wire: `""` for all and for the selected list (host words), else the word of the module as it stands.
    pub scope: String,
}

/// One read: an entry and the scope its page is asked in (also the scope its changes are watched in).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wire {
    /// The number of the source of the layout this read is for (counts from 1); `0` = no layout asked (`probe`, `connect`).
    pub source: u32,
    /// The entry to read.
    pub form: ModuleForm,
    /// The data type of the entry.
    pub data_type: String,
    /// The scope on the wire.
    pub scope: String,
}

/// What [`select`] chose: the reads, and one line for everything it left out or could not place (never silent).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// The reads, in order.
    pub wires: Vec<Wire>,
    /// What was not read or not found, one line each.
    pub notices: Vec<String>,
}

impl Selection {
    /// The notices on stderr, next to the `read_lines` of the module.
    pub fn say(&self, module: &str) {
        for n in &self.notices {
            eprintln!("fs-starter: forms module={module} {n}");
        }
    }
}

fn key(e: &Entry) -> String {
    format!(
        "{}@{}.{}",
        e.form.form.name, e.form.form.major, e.form.form.minor
    )
}

/// The reads for `wanted` out of `entries`: each wanted source takes every entry of ITS type (in the order of the file), with its
/// own scope and its own number, so two sources of the same type but another scope are two reads. A source whose type no entry has is
/// said in the notices.
///
/// Only a source that names no type (a caller without a layout: `probe`, `connect`, a view made by hand) is placed by
/// [`unwished`]: the stop-gap that reads a form of several types only for the primary type, in scope `""`.
#[must_use]
pub fn select(entries: &[Entry], wanted: &[Wanted]) -> Selection {
    let mut out = Selection::default();
    if wanted.is_empty() {
        unwished(entries, 0, &mut out);
        return out;
    }
    for w in wanted {
        if w.data_type.is_empty() {
            unwished(entries, w.n, &mut out);
            continue;
        }
        let mut found = false;
        for e in entries.iter().filter(|e| e.data_type == w.data_type) {
            found = true;
            let one = wire(e, w.n, &w.scope);
            if !out.wires.contains(&one) {
                out.wires.push(one);
            }
        }
        if !found {
            out.notices.push(format!(
                "source {} type {} has no entry in forms.json; not read",
                w.n, w.data_type
            ));
        }
    }
    out
}

fn wire(e: &Entry, source: u32, scope: &str) -> Wire {
    Wire {
        source,
        form: e.form.clone(),
        data_type: e.data_type.clone(),
        scope: scope.to_owned(),
    }
}

/// A form with ONE type is read once in scope `""`. A form the module declares for several types (planning: `rows@1.0.0` for `Lane`
/// and `Task`) is read only for the PRIMARY type of the module (the type most entries of the file have); with no single most frequent
/// type, or when the primary type is not one of the form's, the form is not read. Every entry left out is said in the notices.
fn unwished(entries: &[Entry], source: u32, out: &mut Selection) {
    let primary = primary_type(entries);
    let mut seen: Vec<String> = Vec::new();
    for e in entries {
        let k = key(e);
        if seen.contains(&k) {
            continue;
        }
        seen.push(k.clone());
        let mut types: Vec<&str> = Vec::new();
        for x in entries.iter().filter(|x| key(x) == k) {
            if !types.contains(&x.data_type.as_str()) {
                types.push(&x.data_type);
            }
        }
        let chosen = if types.len() == 1 {
            Some(e)
        } else {
            entries
                .iter()
                .find(|x| key(x) == k && Some(x.data_type.as_str()) == primary)
        };
        if let Some(c) = chosen {
            let one = wire(c, source, "");
            if !out.wires.contains(&one) {
                out.wires.push(one);
            }
        }
        if types.len() > 1 {
            let listed = types.join(", ");
            out.notices.push(match chosen {
                Some(c) => format!(
                    "{k} is declared for {listed}; without a source type only {} is read",
                    c.data_type
                ),
                None => format!("{k} is declared for {listed}; without a source type not read"),
            });
        }
    }
}

/// The type most entries have; `None` when two types lead.
fn primary_type(entries: &[Entry]) -> Option<&str> {
    let mut counts: Vec<(&str, usize)> = Vec::new();
    for e in entries {
        match counts.iter_mut().find(|(t, _)| *t == e.data_type) {
            Some((_, n)) => *n += 1,
            None => counts.push((&e.data_type, 1)),
        }
    }
    let top = counts.iter().map(|(_, n)| *n).max()?;
    let mut leaders = counts.iter().filter(|(_, n)| *n == top);
    let first = leaders.next()?;
    leaders.next().is_none().then_some(first.0)
}

#[cfg(test)]
#[path = "entries_tests.rs"]
mod tests;
