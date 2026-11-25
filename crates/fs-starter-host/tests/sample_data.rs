//! `--sample-data` of the host (Paket 4b): the sources of a description are sample modules made from the description, held in this
//! process. Three conditions of C1:
//! A. only the flag asks for them (without it the same description goes the road of a real module);
//! B. the window says "Sample data" (the records are sample records, `is_sample`), and a real module never does;
//! C. a command stays in the process: no module is asked (no door knocked), no network socket is made, no registry folder is read, the change is in the memory of the window.
//!
//! Pattern: none - tests through the port `open` of `fs-forms-run`, as the window calls it. What they can and cannot see: the doors count every
//! connection to the socket of a module (also one that is closed again), the net count sees tcp and udp sockets; a unix socket that is not a
//! door of a module (a `socketpair`, a connect to some other path) is NOT seen, and no test proves "no network" in general (the structure
//! does: the sample code has no outgoing port).
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

/// Sources that know no module at all and whose folders do not exist: a real read cannot succeed and nothing of a registry is read, so
/// whatever works came from the sample.
fn no_modules() -> StarterSources {
    let modules = Modules::load(
        Path::new("/nonexistent/bundled"),
        Path::new("/nonexistent/system"),
        &SystemPackaged::new(),
    );
    StarterSources::new(modules, Borders::default())
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

/// The network sockets of this process (tcp, udp; `/proc/net/*` against `/proc/self/fd`). A unix socket is NOT counted here: the
/// private `socketpair` of every async runtime has no path and cannot be told from a client that connected out (a `connect` shows no
/// path in `/proc/net/unix`), so the way to a module is watched by [`Doors`] instead.
fn net_sockets() -> usize {
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
    let inodes = |name: &str| -> Vec<String> {
        std::fs::read_to_string(format!("/proc/net/{name}"))
            .unwrap_or_default()
            .lines()
            .skip(1)
            .filter_map(|l| l.split_whitespace().nth(9).map(str::to_owned))
            .collect()
    };
    let net: Vec<String> = ["tcp", "tcp6", "udp", "udp6"]
        .iter()
        .flat_map(|n| inodes(n))
        .collect();
    mine.iter().filter(|i| net.contains(i)).count()
}

/// The doors of the modules a window could connect to: for each name a listener on exactly the socket `fs_rpc::default_socket(name)`
/// gives (in a runtime folder of this test), so every try to reach that module is counted, also one that is closed again before the
/// test looks (what a count of the open sockets would miss). Counts connections, never closes one.
struct Doors {
    _dir: tempfile::TempDir,
    listeners: Vec<std::os::unix::net::UnixListener>,
}

impl Doors {
    /// Doors for `names`. Sets `XDG_RUNTIME_DIR` of the process once (every test of this file holds [`SPY`], so they run one after the other).
    fn new(names: &[&str]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        // SAFETY: every test of this binary holds SPY, so none runs while the variable is written (the others read the environment, e.g. the locale).
        unsafe { std::env::set_var("XDG_RUNTIME_DIR", dir.path()) };
        let listeners = names
            .iter()
            .map(|n| {
                let path = fs_rpc::default_socket(n).expect("a socket path");
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                let l = std::os::unix::net::UnixListener::bind(path).unwrap();
                l.set_nonblocking(true).unwrap();
                l
            })
            .collect();
        Self {
            _dir: dir,
            listeners,
        }
    }

    /// How many connections the doors took since the last look.
    fn callers(&self) -> usize {
        self.listeners
            .iter()
            .map(|l| std::iter::from_fn(|| l.accept().ok()).count())
            .sum()
    }
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
    let _one = SPY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let sources = no_modules();
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
    let _one = SPY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let sources = no_modules();
    assert!(opened(BOARD, true, &sources).records.is_sample());
    assert!(!opened(BOARD, false, &sources).records.is_sample());
    // the wording of the status line exists (fs-forms-run holds it; the key must not vanish)
    let said = words(&session_locale()).plain("host-status-sample");
    assert!(said != "host-status-sample" && !said.is_empty(), "{said}");
}

/// A description without a layout (a view from memory) cannot make test data: said, never invented.
#[test]
fn without_a_description_the_flag_says_so() {
    let _one = SPY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let sources = no_modules();
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

/// C: a write runs in the process. No module is known and the folders of the modules do not exist (so no service could answer and
/// nothing of the registry is read), nobody knocks at the door of a module (`planning`, the contract of the description, and `sample`),
/// no network socket is made, the card moved in the window, and the undo takes it back. The doors are checked while the records
/// still live (the hold of the module is open) and after the last command.
#[test]
fn a_command_stays_in_the_process_no_door_knocked_no_net_socket() {
    let _one = SPY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let doors = Doors::new(&["planning", "sample", "tasks"]);
    let sources = no_modules();
    let net = net_sockets();
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
    assert_eq!(doors.callers(), 0, "nobody knocked at a door of a module");
    assert_eq!(net_sockets(), net, "the sample made no network socket");
    drop(records);
}

/// The counter-checks of the spies: each SEES what it is there for, so "nothing" above means something. A client that connects to a
/// door and closes at once is seen (a count of open sockets would not), and so is a network socket.
#[test]
fn the_spies_see_a_knock_at_a_door_and_a_network_socket() {
    let _one = SPY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let doors = Doors::new(&["planning"]);
    assert_eq!(doors.callers(), 0);
    let path = fs_rpc::default_socket("planning").unwrap();
    drop(std::os::unix::net::UnixStream::connect(path).unwrap());
    assert_eq!(doors.callers(), 1, "a connect that is closed again");
    let before = net_sockets();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    assert_eq!(net_sockets(), before + 1);
    drop(listener);
    assert_eq!(net_sockets(), before);
}
