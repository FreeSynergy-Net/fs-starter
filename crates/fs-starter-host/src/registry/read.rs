//! Reading the command set of a module: `<modules>/<module>/catalog/commands.json`, the file `fs-capture` ships (`catalog/commands.json`
//! @a9ca621). Mandatory: `module` (= the folder name), `version` (`Major.Minor`), `contract` (= module), `scope` (`own-contract`) and a
//! non-empty `commands`; per command `id`, `capability` (in the name space of the module), `access` (`read`/`write`/`delete`) and
//! `help` (`topic`, `text.en`, `text.de`). `call` is kept as text and never run.
//!
//! Pattern: Reader (one parser from the file's text to the value types; a file that cannot be read is a coded refusal).

use std::fs;
use std::io::Read;
use std::path::Path;

use fs_components_commands::Command;
use serde_json::Value;

use super::model::{Access, Help, Item, ModuleSet, Refusal};

fn bad(c: &str) -> Refusal {
    Refusal::Unreadable(c.to_owned())
}

fn text<'a>(v: &'a Value, key: &str, cause: &str) -> Result<&'a str, Refusal> {
    v.get(key).and_then(Value::as_str).ok_or_else(|| bad(cause))
}

/// Read the exchange file of a set of the module in the folder `folder`.
///
/// # Errors
/// [`Refusal::Unreadable`] with the cause (`json`, `module`, `version`, `contract`, `scope`, `commands`, `command-id`,
/// `command-capability`, `command-access`, `command-help`); [`Refusal::ModuleName`] if `module` is not the folder name.
pub fn parse(text_in: &str, folder: &str) -> Result<ModuleSet, Refusal> {
    let doc: Value = serde_json::from_str(text_in).map_err(|_| bad("json"))?;
    let module = text(&doc, "module", "module")?;
    if module != folder {
        return Err(Refusal::ModuleName(module.to_owned()));
    }
    let version = text(&doc, "version", "version")?;
    if !super::valid_version(version) {
        return Err(bad("version"));
    }
    if text(&doc, "contract", "contract")? != module {
        return Err(bad("contract"));
    }
    if text(&doc, "scope", "scope")? != "own-contract" {
        return Err(bad("scope"));
    }
    let list = doc
        .get("commands")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .ok_or_else(|| bad("commands"))?;
    let space = format!("{module}.");
    let mut items = Vec::new();
    for c in list {
        let id = text(c, "id", "command-id")?;
        if !text(c, "capability", "command-capability")?.starts_with(&space) {
            return Err(bad("command-capability"));
        }
        let access = Access::parse(text(c, "access", "command-access")?)
            .ok_or_else(|| bad("command-access"))?;
        let h = c.get("help").ok_or_else(|| bad("command-help"))?;
        let t = h.get("text").ok_or_else(|| bad("command-help"))?;
        let help = Help {
            topic: text(h, "topic", "command-help")?.to_owned(),
            en: text(t, "en", "command-help")?.to_owned(),
            de: text(t, "de", "command-help")?.to_owned(),
        };
        items.push(Item {
            command: Command::new(id, help.topic.as_str()),
            access,
            help,
            call: c.get("call").and_then(Value::as_str).map(str::to_owned),
        });
    }
    Ok(ModuleSet {
        module: module.to_owned(),
        version: version.to_owned(),
        items,
    })
}

/// The biggest file read (a set is a few KiB).
pub(crate) const MAX_BYTES: u64 = 256 * 1024;

/// Is there a file to read in the module folder? `Ok(false)`: no (not a module folder). A symlink (the folder, `catalog` or the file),
/// something that is not a plain file, or a file over [`MAX_BYTES`] is refused, never followed.
pub(crate) fn check(folder: &Path, file: &Path) -> Result<bool, Refusal> {
    let link = |p: &Path| fs::symlink_metadata(p);
    let meta = link(folder).map_err(|_| bad("read-failed"))?;
    if meta.file_type().is_symlink() {
        return Err(bad("symlink"));
    }
    if !meta.is_dir() {
        return Ok(false);
    }
    if link(&folder.join("catalog")).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(bad("symlink"));
    }
    let Ok(m) = link(file) else { return Ok(false) };
    if m.file_type().is_symlink() {
        return Err(bad("symlink"));
    }
    if !m.is_file() {
        return Ok(false);
    }
    if m.len() > MAX_BYTES {
        return Err(bad("too-large"));
    }
    Ok(true)
}

/// Open the file without following a symlink: on unix `O_NOFOLLOW` makes the open itself fail if the last part was swapped for a link
/// after [`check`]. Windows has no such flag for this call: there only the check before applies (documented limit; the folder belongs
/// to the installer).
pub(crate) fn open_plain(file: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(file)
}

/// Read the file of one module (checked by [`check`]) as the set of the module `name` (the folder name): opened without following a
/// link and read at most [`MAX_BYTES`] + 1 bytes, so a file swapped after the check is neither followed nor read without a limit.
pub(super) fn read_folder(file: &Path, name: &str) -> Result<ModuleSet, Refusal> {
    let mut bytes = Vec::new();
    open_plain(file)
        .and_then(|f| f.take(MAX_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|_| bad("read-failed"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(bad("too-large"));
    }
    let text = String::from_utf8(bytes).map_err(|_| bad("read-failed"))?;
    parse(&text, name)
}
