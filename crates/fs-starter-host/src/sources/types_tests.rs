//! The reader of `catalog/types.bin`: absent is no error, every flaw is a named cause, a good file gives the descriptions.
//!
//! Pattern: Table of cases (each cause one case).

use super::{Broken, read};

fn module(bytes: Option<&[u8]>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("catalog")).unwrap();
    if let Some(b) = bytes {
        std::fs::write(dir.path().join("catalog/types.bin"), b).unwrap();
    }
    dir
}

#[test]
fn a_module_without_the_file_has_no_catalog_and_no_error() {
    assert_eq!(read(module(None).path()).map(|r| r.is_none()), Ok(true));
}

#[test]
fn a_file_that_is_no_message_is_damaged() {
    let got = read(module(Some(b"not a capnp message at all")).path());
    assert!(
        matches!(
            got,
            Err(Broken {
                cause: "damaged",
                ..
            })
        ),
        "{got:?}"
    );
}

#[test]
fn a_file_over_the_border_is_too_large() {
    let big = vec![0u8; 300 * 1024];
    let got = read(module(Some(&big)).path());
    assert!(
        matches!(
            got,
            Err(Broken {
                cause: "too-large",
                ..
            })
        ),
        "{got:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_link_is_refused_and_not_followed() {
    let dir = module(None);
    let real = dir.path().join("real.bin");
    std::fs::write(&real, b"x").unwrap();
    std::os::unix::fs::symlink(&real, dir.path().join("catalog/types.bin")).unwrap();
    let got = read(dir.path());
    assert!(
        matches!(
            got,
            Err(Broken {
                cause: "symlink",
                ..
            })
        ),
        "{got:?}"
    );
}
