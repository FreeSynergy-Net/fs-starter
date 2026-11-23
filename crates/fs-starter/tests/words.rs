//! K1-K9 of ENTWURF section 5: the words, the names, the version.
mod support;

use std::ffi::OsString;

use fs_starter::{
    ActorName, FrontendId, InterfaceReq, InterfaceVersion, ProgramName, ResumeToken, StartWords,
    StarterCode, Version, Words, parse, parse_version,
};
use fs_types::coded::Coded;
use support::{fid, os, text};

fn usage_word(words: &[&str]) -> String {
    let e = parse(&os(words)).expect_err("must be refused");
    assert_eq!(e.code(), StarterCode::Usage, "{words:?}");
    text(&e, "word").expect("word parameter")
}

fn start(words: &[&str]) -> StartWords {
    match parse(&os(words)).expect("parses") {
        Words::Start(s) => s,
        other => panic!("not a start: {other:?}"),
    }
}

/// K1: `--version`, `-V` alone; a second word is `usage{word}`.
#[test]
fn k1_version() {
    assert_eq!(parse(&os(&["--version"])), Ok(Words::Version));
    assert_eq!(parse(&os(&["-V"])), Ok(Words::Version));
    assert_eq!(usage_word(&["--version", "x"]), "x");
}

/// K2: `--help`, `-h` alone; a second word is `usage{word}`.
#[test]
fn k2_help() {
    assert_eq!(parse(&os(&["--help"])), Ok(Words::Help));
    assert_eq!(parse(&os(&["-h"])), Ok(Words::Help));
    assert_eq!(usage_word(&["-h", "y"]), "y");
}

/// K3: `serve` hands the rest on byte for byte, also a word that is not UTF-8.
#[cfg(unix)]
#[test]
fn k3_serve_rest_unchanged() {
    use std::os::unix::ffi::OsStringExt;
    let mut words = os(&["serve", "--socket", "s", "--data", "d"]);
    words.push(OsString::from_vec(vec![0xff, 0xfe, b'x']));
    let Ok(Words::Serve(rest)) = parse(&words) else {
        panic!("not serve")
    };
    assert_eq!(rest, words[1..].to_vec());
}

/// K4: `mcp --as NAME` and its refusals.
#[test]
fn k4_mcp() {
    let Ok(Words::Mcp { actor }) = parse(&os(&["mcp", "--as", "claude-strategie"])) else {
        panic!("not mcp")
    };
    assert_eq!(actor.as_str(), "claude-strategie");
    for words in [&["mcp"][..], &["mcp", "--as"][..]] {
        let e = parse(&os(words)).expect_err("actor missing");
        assert_eq!(e, Coded::new(StarterCode::ActorMissing), "{words:?}");
    }
    assert_eq!(usage_word(&["mcp", "--as", "Bad_Name"]), "Bad_Name");
    assert_eq!(usage_word(&["mcp", "--as", "a", "extra"]), "extra");
    assert_eq!(usage_word(&["mcp", "--to", "a"]), "--to");
}

/// K5: the start options, each at most once, with a value, not both `--fresh` and `--resumed`.
#[test]
fn k5_start_options() {
    let s = start(&["--frontend", "classic2", "--socket", "/s", "--fresh"]);
    assert_eq!(s.frontend, Some(fid("classic2")));
    assert_eq!(s.socket.as_deref(), Some(std::path::Path::new("/s")));
    assert!(s.fresh && s.resumed.is_none());
    let s = start(&["--resumed", "t"]);
    assert_eq!(
        s.resumed.as_ref().map(|t| t.as_os_str().to_owned()),
        Some(OsString::from("t"))
    );
    assert_eq!(start(&[]), StartWords::default());

    assert_eq!(
        usage_word(&["--frontend", "a", "--frontend", "b"]),
        "--frontend"
    );
    assert_eq!(
        usage_word(&["--socket", "/a", "--socket", "/b"]),
        "--socket"
    );
    assert_eq!(usage_word(&["--fresh", "--fresh"]), "--fresh");
    assert_eq!(usage_word(&["--frontend"]), "--frontend");
    assert_eq!(usage_word(&["--socket", "--x"]), "--socket");
    assert_eq!(usage_word(&["--frontend", "core"]), "--frontend");
    assert_eq!(usage_word(&["--fresh", "--resumed", "t"]), "--resumed");
    assert_eq!(usage_word(&["--resumed", "t", "--fresh"]), "--fresh");
    assert_eq!(usage_word(&["--nope"]), "--nope");
}

/// K6: the token is never the `word` of a usage error and never shows up in `Debug`.
#[test]
fn k6_token_hidden() {
    let e = parse(&os(&["--resumed", "tok1", "--resumed", "tok2"])).expect_err("repeated");
    assert_eq!(text(&e, "word").as_deref(), Some("--resumed"));
    let shown = format!("{} {e:?}", e.clone().erase());
    assert!(!shown.contains("tok"), "{shown}");
    let token = ResumeToken::parse(OsString::from("tok1").as_os_str()).expect("token");
    assert_eq!(format!("{token:?}"), "ResumeToken(..)");
    let s = start(&["--resumed", "tok1"]);
    assert!(!format!("{s:?}").contains("tok1"));
    assert!(ResumeToken::parse(OsString::new().as_os_str()).is_none());
}

/// K7: names.
#[test]
fn k7_names() {
    for ok in ["simple", "classic2"] {
        assert!(FrontendId::parse(ok).is_some(), "{ok}");
        assert!(ActorName::parse(ok).is_some(), "{ok}");
    }
    let long = "a".repeat(65);
    for bad in ["A", "-a", "a-", "a--b", long.as_str(), ""] {
        assert!(FrontendId::parse(bad).is_none(), "{bad}");
    }
    assert!(FrontendId::parse(&"a".repeat(64)).is_some());
    for reserved in [
        "core",
        "server",
        "client",
        "app",
        "bin",
        "cli",
        "engine",
        "testserver",
    ] {
        assert!(FrontendId::parse(reserved).is_none(), "{reserved}");
    }
    assert!(ProgramName::parse("fs-x").is_none());
    assert!(ProgramName::parse("activities").is_some());
}

/// K8: contract requirements and who serves whom (`name@<req>`, one way of writing).
#[test]
fn k8_contract_req() {
    let offered: InterfaceVersion = "history@0.1.0".parse().expect("parses");
    let wants = |s: &str| s.parse::<InterfaceReq>().expect("parses").matches(&offered);
    assert!(wants("history@^0.1.0") && wants("history@>=0.0.1, <0.2.0"));
    assert!(!wants("history@^0.2.0") && !wants("history@^1.0.0") && !wants("people@^0.1.0"));
    // the blank form of before is no need at all
    for bad in ["history", "history 0.1", "History@^0.1.0"] {
        assert!(bad.parse::<InterfaceReq>().is_err(), "{bad}");
    }
    // read by the grammar, but not a full need (the manifest reader refuses it: three numbers in every comparison)
    for partial in ["history@0.1", "history@0.x", "history@*"] {
        assert!(!partial.parse::<InterfaceReq>().expect("grammar").is_full());
    }
    assert_eq!(offered.to_string(), "history@0.1.0");
}

/// K9: the version of a program, three numbers.
#[test]
fn k9_version() {
    assert_eq!(parse_version("0.0.2"), Some(Version::new(0, 0, 2)));
    for bad in [
        "v0.0.2",
        "0.0",
        "0.0.2.1",
        "0..2",
        "",
        "0.0.2-rc.1",
        "0.0.2+b",
    ] {
        assert_eq!(parse_version(bad), None, "{bad}");
    }
    assert!(Version::new(0, 0, 2) < Version::new(0, 1, 0));
}
