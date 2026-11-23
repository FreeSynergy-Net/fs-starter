//! Shared by the tests: the host binary run with words, and the window drawn through it (`--picture`) and read back as pixels.

#![allow(dead_code)] // each test file uses a part

pub mod install;
pub mod sets;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct Picture {
    pub w: u32,
    pub h: u32,
    pub rgb: Vec<u8>,
}

/// The blueprint folder of fs-tasks (a copy, `tests/data/fs-tasks` of the repository).
pub fn tasks() -> String {
    format!("{}/../../tests/data/fs-tasks", env!("CARGO_MANIFEST_DIR"))
}

/// Where pictures go: `FS_PICTURE_OUT` if set, else `pictures/` below the test folder of the target (kept, so they can be looked
/// at; the target is the one place the tests write to).
pub fn out_dir() -> ((), PathBuf) {
    let dir = std::env::var_os("FS_PICTURE_OUT").map_or_else(
        || PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("pictures"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&dir).unwrap();
    ((), dir)
}

/// The host with `args` in a clean environment (no session, no real user folders: HOME and the XDG folders are temp ones).
pub fn host(args: &[&str], scheme: &str) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_fs-starter"))
        .args(args)
        .env("LANG", "en_US.UTF-8")
        .env("FS_COLOR_SCHEME", scheme)
        .env("HOME", home.path())
        .env("XDG_RUNTIME_DIR", home.path())
        .env("XDG_DATA_HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env_remove("FS_RENDER_SCHEME")
        .env_remove("LC_ALL")
        .env_remove("LC_MESSAGES")
        .output()
        .unwrap()
}

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The window drawn to `path`; `None` (a message) where the machine has no graphics adapter.
pub fn draw(args: &[&str], scheme: &str, path: &Path) -> Option<Picture> {
    draw_code(args, scheme, path, 0)
}

/// Like `draw`, for a run that ends with the exit code `code` (the picture is written anyway).
pub fn draw_code(args: &[&str], scheme: &str, path: &Path, code: i32) -> Option<Picture> {
    let mut all: Vec<&str> = args.to_vec();
    let file = path.to_str().unwrap();
    all.extend(["--picture", file]);
    let out = host(&all, scheme);
    let err = text(&out.stderr);
    if out.status.code() != Some(code) {
        assert!(
            err.to_lowercase().contains("graphics") || err.to_lowercase().contains("adapter"),
            "failed for another reason: {err}"
        );
        eprintln!("skipped: no graphics adapter ({err})");
        return None;
    }
    Some(read_png(path))
}

/// The pixels of a PNG the host wrote.
pub fn read_png(path: &Path) -> Picture {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let step = info.color_type.samples();
    let rgb = buf[..info.buffer_size()]
        .chunks(step)
        .flat_map(|p| p[..3].to_vec())
        .collect();
    Picture {
        w: info.width,
        h: info.height,
        rgb,
    }
}

#[allow(clippy::cast_precision_loss)] // a picture has far fewer than 2^52 samples
pub fn mean(p: &Picture) -> f64 {
    p.rgb.iter().map(|b| f64::from(*b)).sum::<f64>() / p.rgb.len() as f64
}

/// Whether every pixel of the box has the colour of its first one.
pub fn uniform(p: &Picture, x: std::ops::Range<u32>, y: std::ops::Range<u32>) -> bool {
    let at = |x: u32, y: u32| {
        let i = ((y * p.w + x) * 3) as usize;
        [p.rgb[i], p.rgb[i + 1], p.rgb[i + 2]]
    };
    let first = at(x.start, y.start);
    y.clone().all(|yy| x.clone().all(|xx| at(xx, yy) == first))
}

/// The data package `fs-blueprint new` wrote for fs-forms (a copy, `tests/data/fs-forms-package`).
pub fn forms_package() -> String {
    format!(
        "{}/../../tests/data/fs-forms-package",
        env!("CARGO_MANIFEST_DIR")
    )
}
