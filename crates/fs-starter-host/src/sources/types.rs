//! What a module says of its data types: `<module>/catalog/types.bin`, the answer of `TypeCatalog.describe` (fs-types `types.capnp`)
//! as one canonical Cap'n Proto message, opened as carefully as `forms.json` (no symlink, at most 256 KiB), read by the one reader
//! `TypeRegistry::read` (it validates). Each form read gets the description of its data type (`FormData::with_type_description`), so a
//! head is the word `person.<field>` of the module and not the name of the field.
//!
//! Pattern: Reader + Decorator – the file in, a registry out; the rows of a read are decorated with the description of their type.
//! The file instead of the call `TypeCatalog.describe` is a deviation of the transport only (the bytes are the same answer).

use std::io::Read;
use std::path::Path;

use capnp::message::ReaderOptions;
use fs_forms_run::FormRecords;
use fs_types::describe::TypeRegistry;
use fs_types::types_capnp::type_catalog::describe_results;

use crate::registry::read::{MAX_BYTES, check, open_plain};

/// The traversal border of the message: 256 KiB of bytes are 32768 words (a message cannot say more than the file holds, twice).
const TRAVERSAL_LIMIT_WORDS: usize = 64 * 1024;

/// A catalog that cannot be used: the cause word (`symlink`, `too-large`, `read-failed`, `damaged`, `invalid`) and what the reader said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Broken {
    /// The cause word.
    pub cause: &'static str,
    /// The words of the reader (empty for a file problem).
    pub reason: String,
}

fn broken(cause: &'static str, reason: &str) -> Broken {
    Broken {
        cause,
        reason: reason.to_owned(),
    }
}

/// The type catalog of the module in `folder`; `None` when the module has no `catalog/types.bin` (an older module: no words, no error).
///
/// # Errors
/// [`Broken`]: the file is a link, too large, unreadable, not a message, or a description does not validate.
pub fn read(folder: &Path) -> Result<Option<TypeRegistry>, Broken> {
    let file = folder.join("catalog").join("types.bin");
    match check(folder, &file) {
        Ok(false) => return Ok(None),
        Ok(true) => {}
        Err(crate::registry::Refusal::Unreadable(c)) => {
            return Err(broken(
                if c == "symlink" {
                    "symlink"
                } else {
                    "too-large"
                },
                "",
            ));
        }
        Err(_) => return Err(broken("read-failed", "")),
    }
    let mut bytes = Vec::new();
    open_plain(&file)
        .and_then(|f| f.take(MAX_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|e| broken("read-failed", &e.to_string()))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(broken("too-large", ""));
    }
    let mut options = ReaderOptions::new();
    options.traversal_limit_in_words(Some(TRAVERSAL_LIMIT_WORDS));
    let mut slice = bytes.as_slice();
    let message = capnp::serialize::read_message(&mut slice, options)
        .map_err(|e| broken("damaged", &e.to_string()))?;
    let root = message
        .get_root::<describe_results::Reader<'_>>()
        .and_then(describe_results::Reader::get_types)
        .map_err(|e| broken("damaged", &e.to_string()))?;
    TypeRegistry::read(root)
        .map(Some)
        .map_err(|e| broken("invalid", &e.to_string()))
}

/// Every form of `records` gets the description of its data type (the type of its `forms.json` entry); a form whose type the catalog
/// does not describe stays as it is (its heads are the field names).
pub fn attach(records: &mut FormRecords, registry: &TypeRegistry) {
    for data in &mut records.forms {
        if let Some(description) = registry.get(&data.data_type) {
            *data = data.clone().with_type_description(description.clone());
        }
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
