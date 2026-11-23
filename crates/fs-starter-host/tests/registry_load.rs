//! What the registry takes in from disk and from ids: bad characters refuse a set, no forged lines, symlinks, big files, hidden folders.

mod support;
use support::sets::{capture, ids, json, modules_dir, set};
use support::{host, text};

use fs_starter_host::registry::{Actor, Refusal, Registry};

#[test]
fn an_id_or_module_with_a_bad_character_refuses_the_whole_set() {
    for bad in [
        "capture.x\ny",
        "capture.x\ty",
        "capture.x y",
        "capture.X",
        "capture.",
        "capture..x",
        "capture",
        "capture.é",
    ] {
        let mut r = Registry::new();
        let why = r
            .add_bundled(Actor::Person, set("capture", &["capture.ok", bad]))
            .unwrap_err();
        assert!(
            why.iter()
                .any(|w| matches!(w, Refusal::BadId(id) if id == bad)),
            "{bad:?}: {why:?}"
        );
        assert_eq!(r.entries().len(), 4, "{bad:?}: nothing is added");
    }
    for module in ["Capture", "cap ture", "cap\nture", "cap.ture", ""] {
        let mut r = Registry::new();
        let mut s = set("capture", &["capture.ok"]);
        s.module = module.to_owned();
        let why = r.add_bundled(Actor::Person, s).unwrap_err();
        assert!(
            why.iter().any(|w| matches!(w, Refusal::ModuleName(_))),
            "{module:?}: {why:?}"
        );
    }
    let mut r = Registry::new();
    let mut s = set("capture", &["capture.ok"]);
    s.version = "0.1\n9.9".to_owned();
    assert!(r.add_bundled(Actor::Person, s).is_err());
}

#[test]
fn an_id_with_a_line_break_forges_no_line_in_list_commands() {
    let evil = "capture.x\nfake.cmd\tapp\t0.0.4\ncapture.y";
    let dir = modules_dir(&[("capture", &json("capture", &[evil]))]);
    let out = host(
        &["--list-commands", "--modules", dir.path().to_str().unwrap()],
        "light",
    );
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    let lines = text(&out.stdout);
    assert_eq!(lines.lines().count(), 4, "{lines}");
    assert!(!lines.contains("fake.cmd"), "{lines}");
    assert!(
        text(&out.stderr).contains("command-bad-id:"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(
        text(&out.stderr).lines().count(),
        1,
        "one line, no forged one"
    );
}

#[test]
fn symlinks_big_files_and_hidden_folders_are_not_read() {
    let dir = modules_dir(&[("capture", &capture())]);
    // a symlink to a good module folder, a symlinked file, a file over the limit, a hidden folder
    let root = dir.path();
    std::os::unix::fs::symlink(root.join("capture"), root.join("linked")).unwrap();
    let catalog = root.join("filelink").join("catalog");
    std::fs::create_dir_all(&catalog).unwrap();
    std::os::unix::fs::symlink(
        root.join("capture/catalog/commands.json"),
        catalog.join("commands.json"),
    )
    .unwrap();
    let big = root.join("big").join("catalog");
    std::fs::create_dir_all(&big).unwrap();
    std::fs::write(big.join("commands.json"), vec![b' '; 300 * 1024]).unwrap();
    let hidden = root.join(".git").join("catalog");
    std::fs::create_dir_all(&hidden).unwrap();
    std::fs::write(
        hidden.join("commands.json"),
        capture().replace("capture", "git"),
    )
    .unwrap();
    let mut r = Registry::new();
    let refused = r.load_bundled(root).unwrap();
    let cause = |m: &str| refused.iter().find(|(n, _)| n == m).map(|(_, w)| w.clone());
    assert_eq!(
        cause("linked"),
        Some(vec![Refusal::Unreadable("symlink".into())])
    );
    assert_eq!(
        cause("filelink"),
        Some(vec![Refusal::Unreadable("symlink".into())])
    );
    assert_eq!(
        cause("big"),
        Some(vec![Refusal::Unreadable("too-large".into())])
    );
    assert_eq!(
        refused.len(),
        3,
        "{refused:?}: the hidden folder is skipped silently"
    );
    assert_eq!(ids(&r).len(), 5, "only capture.shot came in: {:?}", ids(&r));
}
