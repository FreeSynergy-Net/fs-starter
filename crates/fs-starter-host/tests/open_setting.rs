//! E3 (ENTWURF einstellungen-als-daten): the host reads `fs.open.when-open` through `Settings::open(program, Catalog)`.
//! G6: no service, `store-unreachable`, the value is the default `ask`, one visible line, no crash.
//! G7: person `window`, program profile `switch` gives `switch`; an instance lock `ask` gives `ask` with the lock shown.
use std::sync::Arc;

use fs_rpc::SingleTimes;
use fs_settings::{
    Admin, GrantedLockRights, LockGrant, MemoryStore, NoSystem, Place, ProgramName, Raw, Scope,
    SettingCode, SettingKey, Settings, SettingsStore, Value, system_clock,
};
use fs_starter::{StarterSettings, WHEN_OPEN, WhenOpen};
use fs_starter_host::open_setting::{OpenSetting, catalog, connect, read};

fn key() -> SettingKey {
    StarterSettings::key(WHEN_OPEN).expect("key")
}

fn program() -> ProgramName {
    ProgramName::new("tasks").expect("program")
}

fn open(store: &MemoryStore) -> Settings {
    Settings::open(
        program(),
        catalog().expect("catalog"),
        Arc::new(store.clone()),
        Arc::new(NoSystem),
    )
}

fn put(store: &MemoryStore, scope: &Scope, word: &str) {
    store
        .write(
            scope,
            Place::Profile,
            0,
            &key(),
            Some(&Raw::Str(word.to_owned())),
        )
        .expect("written");
}

/// G6: no `fs-settings` program beside the running file and none in the (empty) system folder: nothing to start.
#[test]
fn g6_without_a_service_the_value_is_ask_and_one_line_says_why() {
    let dir = tempfile::tempdir().expect("dir");
    let socket = dir.path().join("settings.sock");
    let running = dir.path().join("fs-starter");
    let usr_bin = dir.path().join("usr-bin"); // an empty folder: the test never sees the `/usr/bin` of the machine
    std::fs::create_dir(&usr_bin).expect("folder");
    let got = connect(
        &program(),
        &socket,
        &running,
        &usr_bin,
        &SingleTimes::default(),
    );
    assert_eq!(got.when_open, WhenOpen::Ask);
    let why = got.notice.as_ref().expect("a notice");
    assert_eq!(why.code(), SettingCode::StoreUnreachable);
    let line = got.line().expect("a visible line");
    assert!(line.starts_with("fs-starter: yellow "), "{line}");
    assert!(
        line.contains("store-unreachable") && line.contains("value=ask"),
        "{line}"
    );
    assert!(
        line.contains("no-server"),
        "the cause is the one of ensure_service: {line}"
    );
    assert_eq!(got.locked, None);
}

/// Without a notice and without a lock there is nothing to say.
#[test]
fn a_default_value_says_nothing() {
    let got = read(&open(&MemoryStore::new()));
    assert_eq!(
        got,
        OpenSetting {
            when_open: WhenOpen::Ask,
            locked: None,
            notice: None
        }
    );
    assert_eq!(got.line(), None);
}

/// G7: person `window`, program profile `switch`: the program's own choice wins.
#[test]
fn g7_the_program_profile_beats_the_person() {
    let store = MemoryStore::new();
    let settings = open(&store);
    put(&store, &Scope::General, "window");
    settings.refresh();
    assert_eq!(
        read(&settings).when_open,
        WhenOpen::Window,
        "the person alone"
    );
    put(&store, &Scope::Program(program()), "switch");
    settings.refresh();
    let got = read(&settings);
    assert_eq!(got.when_open, WhenOpen::Switch);
    assert_eq!(got.locked, None);
}

/// G7: a lock of the instance (`ask`, with a reason) beats person and program, and is shown with its reason.
#[test]
fn g7_an_instance_lock_beats_both_and_is_shown() {
    let store = MemoryStore::new();
    let settings = open(&store);
    put(&store, &Scope::General, "window");
    put(&store, &Scope::Program(program()), "switch");
    let admin = Admin {
        store: Arc::new(store.clone()),
        rights: Arc::new(GrantedLockRights(LockGrant::InstanceAdmin)),
        by: fs_types::BasicValue::parse("https://example.org/people/admin").expect("iri"),
        catalog: catalog().expect("catalog"),
        clock: system_clock,
    };
    admin
        .lock(
            &key(),
            &Value::Choice("ask".to_owned()),
            "ask everyone first",
        )
        .expect("locked");
    settings.refresh();
    let got = read(&settings);
    assert_eq!(got.when_open, WhenOpen::Ask);
    assert_eq!(got.locked.as_deref(), Some("ask everyone first"));
    let line = got.line().expect("the lock is visible");
    assert!(
        line.contains("locked") && line.contains("ask everyone first"),
        "{line}"
    );
}

/// §9.4: with the question open (`ask`) the yellow line says that remembering is not there yet; with a chosen word it is not said.
#[test]
fn the_remember_line_is_said_only_while_the_question_is_asked() {
    let ask = read(&open(&MemoryStore::new()));
    assert!(
        ask.lines()
            .iter()
            .any(|l| l.contains("yellow remember-unavailable")),
        "{:?}",
        ask.lines()
    );
    let store = MemoryStore::new();
    let settings = open(&store);
    put(&store, &Scope::General, "window");
    settings.refresh();
    assert_eq!(read(&settings).lines(), Vec::<String>::new());
}

/// The service socket is named `settings` (`socket_program(false)` of fs-settings-server), the same on both sides.
#[test]
fn the_socket_of_the_person_service_is_named_settings() {
    assert_eq!(fs_starter_host::open_setting::SOCKET_PROGRAM, "settings");
}

/// The folder of the system programs is the one NAMED, never a fixed `/usr/bin` (a hermetic G6): a service program that is there is
/// tried (here one that ends at once: the cause is then not `no-server`), an empty folder says `no-server`.
#[cfg(unix)]
#[test]
fn the_service_program_is_looked_for_in_the_named_folder_only() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("dir");
    let (socket, running) = (dir.path().join("s.sock"), dir.path().join("app/fs-starter"));
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    let exe = bin.join("fs-settings");
    std::fs::write(&exe, "#!/bin/sh\nexit 1\n").expect("script");
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).expect("mode");
    let times = SingleTimes::default();
    let tried = connect(&program(), &socket, &running, &bin, &times);
    let line = tried.line().expect("a line");
    assert!(
        !line.contains("no-server"),
        "the program of the named folder was tried: {line}"
    );
}
