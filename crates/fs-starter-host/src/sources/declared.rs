//! What a module says about its forms: `<module>/catalog/forms.json`, in the same plain way as `commands.json` (the same careful
//! open: no symlink, at most 256 KiB). A list of `{name, version, type, map, fields}`; `map` is channel to field name (or a list of
//! field names), `fields` is field name to the basic type word of the type catalog. Only names (ADR 0097 E4), never a path or a call.
//!
//! Pattern: Reader (text to `ModuleForm`; a file that cannot be read is a cause word, never a half result).

use std::io::Read;
use std::path::Path;

use fs_forms_bind::{Basic, FormRef, ModuleForm};
use serde_json::Value;

use super::range::version;
use crate::registry::read::{MAX_BYTES, check, open_plain};

fn basic(word: &str) -> Option<Basic> {
    use Basic::{
        Bool, Choice, Date, DateTime, Email, Image, Keywords, Link, Number, Reference, Text,
    };
    [
        Text, Email, Number, Bool, Date, DateTime, Choice, Reference, Keywords, Link, Image,
    ]
    .into_iter()
    .find(|b| b.name() == word)
}

fn names(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::String(s) => Some(vec![s.clone()]),
        Value::Array(a) => a.iter().map(|s| s.as_str().map(str::to_owned)).collect(),
        _ => None,
    }
}

/// The forms of the text of a `forms.json`.
///
/// # Errors
/// A cause word: `json`, `list`, `name`, `version`, `map`, `fields`, `basic`.
pub fn parse(text: &str) -> Result<Vec<ModuleForm>, &'static str> {
    let doc: Value = serde_json::from_str(text).map_err(|_| "json")?;
    let list = doc.as_array().ok_or("list")?;
    let mut out = Vec::new();
    for f in list {
        let name = f.get("name").and_then(Value::as_str).ok_or("name")?;
        let (major, minor) = f
            .get("version")
            .and_then(Value::as_str)
            .and_then(version)
            .ok_or("version")?;
        let map = f
            .get("map")
            .and_then(Value::as_object)
            .ok_or("map")?
            .iter()
            .map(|(c, v)| names(v).map(|n| (c.clone(), n)).ok_or("map"))
            .collect::<Result<Vec<_>, _>>()?;
        let fields = f
            .get("fields")
            .and_then(Value::as_object)
            .ok_or("fields")?
            .iter()
            .map(|(n, w)| {
                w.as_str()
                    .and_then(basic)
                    .map(|b| (n.clone(), b))
                    .ok_or("basic")
            })
            .collect::<Result<Vec<_>, _>>()?;
        out.push(ModuleForm {
            form: FormRef {
                name: name.to_owned(),
                major,
                minor,
            },
            map,
            fields,
        });
    }
    Ok(out)
}

/// The forms of the module in `folder` (`catalog/forms.json`). A module without the file offers no form (an empty list).
///
/// # Errors
/// A cause word: the ones of [`parse`], `symlink` / `too-large` / `read-failed` for the file.
pub fn read(folder: &Path) -> Result<Vec<ModuleForm>, &'static str> {
    let file = folder.join("catalog").join("forms.json");
    match check(folder, &file) {
        Ok(false) => return Ok(Vec::new()),
        Ok(true) => {}
        Err(crate::registry::Refusal::Unreadable(c)) => {
            return Err(if c == "symlink" {
                "symlink"
            } else {
                "too-large"
            });
        }
        Err(_) => return Err("read-failed"),
    }
    let mut bytes = Vec::new();
    open_plain(&file)
        .and_then(|f| f.take(MAX_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|_| "read-failed")?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("too-large");
    }
    parse(&String::from_utf8(bytes).map_err(|_| "read-failed")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"[{"name":"tree","version":"1.0","type":"Group",
        "map":{"nodes":"Group","label":["name"]},"fields":{"name":"text","mine":"bool"}}]"#;

    #[test]
    fn a_form_is_read_with_its_map_and_its_basic_types() {
        let forms = parse(GOOD).unwrap();
        assert_eq!(forms[0].form.name, "tree");
        assert_eq!((forms[0].form.major, forms[0].form.minor), (1, 0));
        assert!(
            forms[0]
                .map
                .contains(&("label".to_owned(), vec!["name".to_owned()]))
        );
        assert!(forms[0].fields.contains(&("mine".to_owned(), Basic::Bool)));
    }

    #[test]
    fn every_flaw_is_named_not_skipped() {
        for (text, cause) in [
            ("{", "json"),
            ("{}", "list"),
            (r#"[{"version":"1.0","map":{},"fields":{}}]"#, "name"),
            (
                r#"[{"name":"t","version":"x","map":{},"fields":{}}]"#,
                "version",
            ),
            (
                r#"[{"name":"t","version":"1.0","map":[],"fields":{}}]"#,
                "map",
            ),
            (
                r#"[{"name":"t","version":"1.0","map":{"a":3},"fields":{}}]"#,
                "map",
            ),
            (r#"[{"name":"t","version":"1.0","map":{}}]"#, "fields"),
            (
                r#"[{"name":"t","version":"1.0","map":{},"fields":{"a":"blob"}}]"#,
                "basic",
            ),
        ] {
            assert_eq!(parse(text), Err(cause), "{text}");
        }
    }
}
