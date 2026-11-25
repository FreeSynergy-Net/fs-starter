//! The form entries a module declares (`catalog/forms.json`) as a registry by the key (form, type), and the choice of what a window
//! reads: a layout source says `(type, scope)`, and ONLY the entries of that type are read, each with the scope of its source. A form
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

/// What one layout source asks for: the data type and the scope word that goes to the module as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wanted {
    /// The data type of the source (`type` of the `[[source]]`).
    pub data_type: String,
    /// The scope on the wire: `""` for all and for the selected list (host words), else the word of the module as it stands.
    pub scope: String,
}

/// One read: an entry and the scope its page is asked in (also the scope its changes are watched in).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wire {
    /// The entry to read.
    pub form: ModuleForm,
    /// The data type of the entry.
    pub data_type: String,
    /// The scope on the wire.
    pub scope: String,
}

/// The reads for `wanted` out of `entries`: each wanted source takes every entry of its type (in the order of the file), once.
/// Without any wish (a caller that cannot say the sources yet) each form of the file is read once, the first entry of its name and
/// version, in scope `""`: a second entry of the same form (another type) never gets the rows of the first.
#[must_use]
pub fn select(entries: &[Entry], wanted: &[Wanted]) -> Vec<Wire> {
    let wire = |e: &Entry, scope: &str| Wire {
        form: e.form.clone(),
        data_type: e.data_type.clone(),
        scope: scope.to_owned(),
    };
    let mut out: Vec<Wire> = Vec::new();
    if wanted.is_empty() {
        for e in entries {
            if !out.iter().any(|w| w.form.form == e.form.form) {
                out.push(wire(e, ""));
            }
        }
        return out;
    }
    for w in wanted {
        for e in entries.iter().filter(|e| e.data_type == w.data_type) {
            let one = wire(e, &w.scope);
            if !out.contains(&one) {
                out.push(one);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_forms_bind::FormRef;

    fn entry(name: &str, data_type: &str) -> Entry {
        Entry {
            form: ModuleForm {
                form: FormRef {
                    name: name.into(),
                    major: 1,
                    minor: 0,
                },
                map: Vec::new(),
                fields: Vec::new(),
            },
            data_type: data_type.into(),
        }
    }

    fn want(t: &str, s: &str) -> Wanted {
        Wanted {
            data_type: t.into(),
            scope: s.into(),
        }
    }

    #[test]
    fn a_source_takes_only_the_entries_of_its_type_with_its_own_scope() {
        let entries = [entry("rows", "Lane"), entry("rows", "Task")];
        let got = select(
            &entries,
            &[want("Lane", "lanes:assignee"), want("Task", "")],
        );
        let got: Vec<_> = got
            .iter()
            .map(|w| (w.data_type.as_str(), w.scope.as_str()))
            .collect();
        assert_eq!(got, [("Lane", "lanes:assignee"), ("Task", "")]);
    }

    #[test]
    fn a_type_nobody_asks_for_is_not_read() {
        let entries = [entry("rows", "Lane"), entry("rows", "Task")];
        let got = select(&entries, &[want("Task", "")]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].data_type, "Task");
    }

    #[test]
    fn without_a_wish_one_entry_per_form_is_read_in_the_empty_scope() {
        let entries = [
            entry("cards", "Task"),
            entry("rows", "Lane"),
            entry("rows", "Task"),
        ];
        let got = select(&entries, &[]);
        assert_eq!(got.len(), 2);
        assert!(got.iter().all(|w| w.scope.is_empty()));
        assert_eq!(got[1].data_type, "Lane");
    }
}
