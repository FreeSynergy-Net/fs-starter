//! The Windows anchor of the system places: the known folder `FOLDERID_ProgramFiles` + `FreeSynergy` (ENTSCHEID-schnitt (2) rule 6).
//!
//! Never the environment variable `%ProgramFiles%` (the user sets it, so it proves nothing); the shell is asked for the known folder.
//! A doubt is no anchor: the system places are then empty and every path is outside the system (the check closes).
//!
//! Pattern: Value function (reads the system, no state).

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Shell::{FOLDERID_ProgramFiles, SHGetKnownFolderPath};

/// `KF_FLAG_DEFAULT` (the flags of `SHGetKnownFolderPath`, a `u32` here; the constant of the crate is an `i32`).
const KF_FLAG_DEFAULT: u32 = 0;

/// `<ProgramFiles>\FreeSynergy`; `None` when the shell does not tell (never a guess).
pub(crate) fn anchor() -> Option<PathBuf> {
    let mut raw: *mut u16 = std::ptr::null_mut();
    // SAFETY: `raw` is written by the call; on success it is a NUL terminated wide string that this function frees with `CoTaskMemFree`
    // (also on failure the call says to free it).
    let status = unsafe {
        SHGetKnownFolderPath(
            &FOLDERID_ProgramFiles,
            KF_FLAG_DEFAULT,
            std::ptr::null_mut(),
            &raw mut raw,
        )
    };
    let path = if status >= 0 && !raw.is_null() {
        // SAFETY: the string is NUL terminated (see above), `len` stops at the terminator, and the memory is still alive here.
        let mut len = 0;
        while unsafe { *raw.add(len) } != 0 {
            len += 1;
        }
        let wide = unsafe { std::slice::from_raw_parts(raw, len) };
        Some(PathBuf::from(OsString::from_wide(wide)))
    } else {
        None
    };
    if !raw.is_null() {
        unsafe { CoTaskMemFree(raw.cast_const().cast()) };
    }
    path.filter(|p| p.is_absolute())
        .map(|p| p.join("FreeSynergy"))
}
