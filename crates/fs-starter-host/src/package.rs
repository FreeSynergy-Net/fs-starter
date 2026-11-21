//! The data package: the folder `fs-blueprint new` writes (`scaffold.toml`, `packaging/layout/`, `forms/`). The host reads only the
//! three facts it shows (program, skeleton version, the layout the blueprint names); the window of the layout is `fs-forms-run`'s.
//!
//! Pattern: Value (what a package says about itself) with one reader; a package that cannot be read is a coded line, never a guess.

use std::fs;
use std::path::Path;

/// What the package's `scaffold.toml` says about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    /// `[program] name`.
    pub program: String,
    /// `[program] skeleton` (the version of the skeleton the package was made for), empty when not named.
    pub skeleton: String,
    /// `[frontend] layout`, the relative path of the layout file, when the blueprint names one.
    pub layout: Option<String>,
}

/// Read the package at `path`. `Ok(None)`: not a package (a file, or a folder without `scaffold.toml`: another kind of description).
///
/// # Errors
/// The code `package-unreadable` with the cause, when `scaffold.toml` is there but is not TOML or has no `[program] name`.
pub fn read(path: &Path) -> Result<Option<Package>, String> {
    let file = path.join("scaffold.toml");
    if !path.is_dir() || !file.is_file() {
        return Ok(None);
    }
    let bad = |cause: &str| format!("package-unreadable: {}: {cause}", file.display());
    let text = fs::read_to_string(&file).map_err(|_| bad("read-failed"))?;
    let doc: toml::Table = text.parse().map_err(|_| bad("toml"))?;
    let text_of = |table: &str, key: &str| {
        doc.get(table)
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let program = text_of("program", "name").ok_or_else(|| bad("program-name-missing"))?;
    Ok(Some(Package {
        program,
        skeleton: text_of("program", "skeleton").unwrap_or_default(),
        layout: text_of("frontend", "layout"),
    }))
}

/// The line for the person (stderr).
#[must_use]
pub fn line(p: &Package) -> String {
    format!(
        "fs-starter: package {} (skeleton {}), layout {}",
        p.program,
        if p.skeleton.is_empty() {
            "?"
        } else {
            &p.skeleton
        },
        p.layout
            .as_deref()
            .unwrap_or("none named (forms/*.toml is searched)")
    )
}
