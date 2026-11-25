//! `--sample-data` of the host (Paket 4b): the sources of a description are sample modules made from the description, held in this
//! process. Three conditions of C1:
//! A. only the flag asks for them (without it the same description goes the road of a real module);
//! B. the window says "Sample data" (the records are sample records, `is_sample`), and a real module never does;
//! C. a command stays in the process: no socket is made, no module of the registry is needed, the change is in the memory of the window.
//!
//! Pattern: Spy (what the process opened: the sockets of `/proc/self/fd`) + Test through the port `open` of `fs-forms-run`, as the window calls it.
#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;

use fs_components_kanban::Answer;
use fs_forms_exchange::{Format, read_layout};
use fs_forms_layout::{Layout, Limits, OpenCatalog};
use fs_forms_run::{Config, Described, Opened, Records, open, session_locale, view_of, words};
use fs_starter_host::registry::SystemPackaged;
use fs_starter_host::sources::{Limits as Borders, Modules, StarterSources};

/// The counter of sockets is process wide: the tests that look at it (or make one) run one after the other.
static SPY: std::sync::Mutex<()> = std::sync::Mutex::new(());

const BOARD: &str = include_str!("data/board-sample.toml");

fn layout(text: &str) -> Layout {
    read_layout(&mut text.as_bytes(), Path::new("board.toml"), Format::Toml)
        .expect("reads")
        .check(&OpenCatalog, &Limits::declared())
        .expect("checks")
}

/// Sources that know no module at all (an empty folder): a real read cannot succeed, so whatever works came from the sample.
fn no_modules() -> (tempfile::TempDir, StarterSources) {
    let dir = tempfile::tempdir().unwrap();
    let modules = Modules::load(
        dir.path(),
        Path::new("/nonexistent"),
        &SystemPackaged::new(),
    );
    (dir, StarterSources::new(modules, Borders::default()))
}

fn opened(text: &str, flag: bool, sources: &StarterSources) -> Opened {
    let layout = layout(text);
    let view = view_of(&layout, "en");
    let c = Config {
        sample_data: flag,
        ..Config::default()
    };
    let texts = words(&session_locale());
    open(
        &c,
        &view,
        &Described::of(Some(&layout)),
        &texts,
        Path::new("x"),
        sources,
    )
}

/// The sockets of this process that reach somewhere: a network socket (tcp, udp) or a unix socket with a path (`/proc/net/*`). The
/// private pair of the async runtime (a `socketpair` without a path) is no way out of the process and is not counted.
fn sockets() -> usize {
    let mine: Vec<String> = std::fs::read_dir("/proc/self/fd")
        .expect("procfs")
        .flatten()
        .filter_map(|e| std::fs::read_link(e.path()).ok())
        .filter_map(|t| {
            let t = t.to_string_lossy().into_owned();
            t.strip_prefix("socket:[")
                .and_then(|r| r.strip_suffix(']'))
                .map(str::to_owned)
        })
        .collect();
    let table = |name: &str, inode: usize, path: bool| -> Vec<String> {
        std::fs::read_to_string(format!("/proc/net/{name}"))
            .unwrap_or_default()
            .lines()
            .skip(1)
            .filter_map(|l| {
                let f: Vec<&str> = l.split_whitespace().collect();
                (!path || f.len() > 7).then(|| f.get(inode).map(|s| (*s).to_owned()))?
            })
            .collect()
    };
    let reaching: Vec<String> = [
        table("tcp", 9, false),
        table("tcp6", 9, false),
        table("udp", 9, false),
        table("udp6", 9, false),
        table("unix", 6, true),
    ]
    .concat();
    mine.iter().filter(|i| reaching.contains(i)).count()
}

fn cards(records: &Arc<dyn Records>) -> Vec<(String, Vec<String>)> {
    let forms = records.forms();
    let board = forms
        .iter()
        .find(|f| f.declared.form.name == "cards-in-columns")
        .expect("the board form");
    board
        .rows
        .iter()
        .map(|r| {
            let col = r
                .values
                .iter()
                .find(|(c, _)| c == "column")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            (r.key.clone(), col)
        })
        .collect()
}

/// A: with the flag the data come from the sample and `[sample]` decides (5 records); a description without the section gets the defaults.
#[test]
fn with_the_flag_and_only_with_it_the_window_gets_sample_records() {
    let (_dir, sources) = no_modules();
    let o = opened(BOARD, true, &sources);
    assert!(o.notice.is_none(), "{:?}", o.notice);
    assert!(!o.connected, "sample data are no connected module");
    assert_eq!(cards(&o.records).len(), 5, "the section [sample] says 5");
    let cut = BOARD.find("\n[sample]").expect("the section");
    let plain = opened(&BOARD[..cut], true, &sources);
    assert_eq!(
        cards(&plain.records).len(),
        8,
        "the default of the declaration"
    );

    // the same description WITHOUT the flag goes the road of a real module: here no module is known, and nothing is made up
    let o = opened(BOARD, false, &sources);
    let notice = o.notice.expect("a notice");
    assert_eq!(notice.code, "host-notice-source-unknown");
    assert!(o.records.forms().is_empty(), "no invented rows");
}

/// B: sample records say so (the window puts "Sample data" in its status line), real or missing ones do not.
#[test]
fn the_records_with_the_flag_are_marked_sample_and_without_it_nothing_is() {
    let (_dir, sources) = no_modules();
    assert!(opened(BOARD, true, &sources).records.is_sample());
    assert!(!opened(BOARD, false, &sources).records.is_sample());
    // the wording of the status line exists (fs-forms-run holds it; the key must not vanish)
    let said = words(&session_locale()).plain("host-status-sample");
    assert!(said != "host-status-sample" && !said.is_empty(), "{said}");
}

/// A description without a layout (a view from memory) cannot make test data: said, never invented.
#[test]
fn without_a_description_the_flag_says_so() {
    let (_dir, sources) = no_modules();
    let view = fs_forms_run::View {
        sources: vec!["planning@>=0.0.1, <0.1.0".into()],
        ..fs_forms_run::View::default()
    };
    let c = Config {
        sample_data: true,
        ..Config::default()
    };
    let o = open(
        &c,
        &view,
        &Described::none(),
        &words(&session_locale()),
        Path::new("x"),
        &sources,
    );
    assert_eq!(
        o.notice.expect("a notice").code,
        "host-notice-sample-no-description"
    );
}

/// C: a write runs in the process. No module is known (so no service could answer), no socket is made, the card moved in the window,
/// and the undo takes it back.
#[test]
fn a_command_stays_in_the_process_no_socket_no_module() {
    let _one = SPY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, sources) = no_modules();
    let before = sockets();
    let o = opened(BOARD, true, &sources);
    let records = o.records;
    let (key, columns) = cards(&records).remove(0);
    let to = "column-that-is-not-there-yet".to_owned();
    let answer = records.invoke(
        "cards-in-columns",
        "move",
        Some(&key),
        &[("column".to_owned(), to.clone())],
    );
    assert!(matches!(answer, Answer::Accepted), "{answer:?}");
    let moved = cards(&records)
        .into_iter()
        .find(|(k, _)| *k == key)
        .expect("the card");
    assert_eq!(moved.1, [to], "the card is in its new column");
    assert_ne!(moved.1, columns);
    assert_eq!(records.history().undoable, 1);
    assert!(matches!(records.step_back(false), Answer::Accepted));
    let back = cards(&records)
        .into_iter()
        .find(|(k, _)| *k == key)
        .unwrap();
    assert_eq!(back.1, columns, "undone");
    assert_eq!(sockets(), before, "the sample made no socket");
    assert!(
        fs_rpc::default_socket("sample").is_none_or(|p| !p.exists()),
        "no service socket of a module `sample`"
    );
}

/// The counter-check of the spy: it SEES a socket when one is made, so "unchanged" above means something.
#[test]
fn the_spy_sees_a_socket_when_one_is_made() {
    let _one = SPY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let before = sockets();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    assert_eq!(sockets(), before + 1);
    drop(listener);
    assert_eq!(sockets(), before);
    // a unix socket with a path (the way to a module service) is seen as well
    let dir = tempfile::tempdir().unwrap();
    let unix = std::os::unix::net::UnixListener::bind(dir.path().join("module.sock")).unwrap();
    assert_eq!(sockets(), before + 1);
    drop(unix);
    assert_eq!(sockets(), before);
}
