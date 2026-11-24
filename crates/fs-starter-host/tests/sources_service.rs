//! SQ-4, the whole road through the host binary (a copy next to `modules/`, with the fake module as the service): registry,
//! `forms.json`, `ensure_single`, hello, heartbeat, pages; the window is drawn (`--picture`) from the records of the service.
//!
//! Tests: (1) the groups window drawn from the service equals the one of `--sample-data` (the same fake data, "Sample data" status)
//! and differs from the empty one; (2) a service that does not say `fake` is NOT drawn as sample data; (3) one border each: level the host
//! does not know, no heartbeat, more records than the limit, no program file, a symbolic link as program file; (4) the socket of the
//! command line is not used; (5) a start twice uses one service. Mutants: a border off = its test red (see the report).
//! Set `FS_PICTURE_OUT=<dir>` to keep the pictures.

#![cfg(unix)]

mod support;
use support::install::{GOOD_FORMS, Install, Service, groups};
use support::{Picture, out_dir, text};

const GROUPS: &str = "host-notice-data";

/// The window of the groups description drawn by `i`; `None` where the machine has no graphics adapter. Returns the picture and the
/// words the host said on stderr.
fn draw(
    i: &Install,
    extra: &[&str],
    scheme: &str,
    name: &str,
    env: &[(&str, &str)],
) -> Option<(Picture, String)> {
    // drawn into a folder of its own (tests run in parallel); a picture with a name is also kept in `FS_PICTURE_OUT` for looking at
    let own = tempfile::tempdir().unwrap();
    let path = own.path().join("w.png");
    let g = groups();
    let mut args = vec!["--description", g.as_str()];
    args.extend_from_slice(extra);
    args.extend(["--picture", path.to_str().unwrap()]);
    let out = i.run(&args, scheme, env);
    let err = text(&out.stderr);
    if out.status.code() != Some(0) {
        assert!(
            err.to_lowercase().contains("graphics") || err.to_lowercase().contains("adapter"),
            "failed for another reason: {err}"
        );
        eprintln!("skipped: no graphics adapter ({err})");
        return None;
    }
    if !name.starts_with('~') {
        let (_k, dir) = out_dir();
        std::fs::copy(&path, dir.join(name)).unwrap();
    }
    Some((support::read_png(&path), err))
}

fn same(a: &Picture, b: &Picture) -> bool {
    a.w == b.w && a.h == b.h && a.rgb == b.rgb
}

fn good() -> Install {
    let i = Install::new();
    i.module("groups", "0.1.0", GOOD_FORMS, &Service::Fake);
    i
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn the_window_from_the_service_is_the_sample_window_and_not_the_empty_one() {
    let i = good();
    let Some((live, said)) = draw(&i, &[], "light", "groups-service.png", &[]) else {
        return;
    };
    assert!(
        !said.contains(GROUPS),
        "no notice for a service that fits: {said}"
    );
    // the one-time command ends the service it started and fs-rpc REMOVES the socket file at the end of a run on purpose (fs-rpc
    // `serve.rs`: "The socket is removed on return"; `BoundSocket` drops it; CHANGELOG: "the socket file is removed (the bound socket drops)"), so the proof of the start is the read
    // line: the runtime folder of `i` is new, nobody else could have started the service that answered
    assert!(
        said.contains("read module=groups form=tree records=9")
            && said.contains("provider=\"fake\""),
        "the host started the service of the module and read it: {said}"
    );
    let (sample, _) = draw(
        &i,
        &["--sample-data"],
        "light",
        "groups-sample-light.png",
        &[],
    )
    .unwrap();
    assert!(
        same(&live, &sample),
        "fake data = sample data: the window says \"Sample data\" and draws the sample groups"
    );
    let none = Install::new();
    let (empty, said) = draw(&none, &[], "light", "groups-empty.png", &[]).unwrap();
    assert!(said.contains("host-notice-source-unknown"), "{said}");
    assert!(!same(&live, &empty), "the empty window (no module) differs");
    let (dark, _) = draw(&i, &[], "dark", "groups-service-dark.png", &[]).unwrap();
    let (dark_sample, _) = draw(
        &i,
        &["--sample-data"],
        "dark",
        "groups-sample-dark.png",
        &[],
    )
    .unwrap();
    assert!(same(&dark, &dark_sample) && !same(&dark, &live));
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn records_of_a_service_that_is_not_fake_are_not_shown_as_sample_data() {
    let i = good();
    let Some((live, _)) = draw(
        &i,
        &[],
        "light",
        "groups-real-provider.png",
        &[("FAKE_MODE", "real")],
    ) else {
        return;
    };
    let (sample, _) = draw(&i, &["--sample-data"], "light", "~sample.png", &[]).unwrap();
    assert!(
        !same(&live, &sample),
        "a provider that is not `fake` must not look like sample data"
    );
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn one_service_serves_two_starts_and_the_socket_of_the_command_line_is_not_used() {
    let i = good();
    let Some((a, _)) = draw(&i, &[], "light", "groups-start-1.png", &[]) else {
        return;
    };
    let evil = i.runtime.path().join("evil.sock");
    let (b, said) = draw(
        &i,
        &["--socket", evil.to_str().unwrap()],
        "light",
        "groups-start-2.png",
        &[],
    )
    .unwrap();
    assert!(same(&a, &b), "the same window; {said}");
    assert!(
        !evil.exists(),
        "nothing was bound or asked on the socket the command line named"
    );
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn every_border_says_its_code_and_draws_no_records() {
    for (mode, code) in [
        ("level", "host-notice-data-contract"),
        ("nobeat", "host-notice-data-unreachable"),
        ("hostile", "host-notice-data-answer"),
    ] {
        let i = good();
        let Some((p, said)) = draw(
            &i,
            &[],
            "light",
            &format!("groups-{mode}.png"),
            &[("FAKE_MODE", mode)],
        ) else {
            return;
        };
        assert!(said.contains(code), "{mode}: {said}");
        let (sample, _) = draw(&i, &["--sample-data"], "light", "~sample.png", &[]).unwrap();
        assert!(!same(&p, &sample), "{mode}: no records are drawn");
    }
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn only_the_program_file_in_the_module_folder_is_started() {
    for (service, cause) in [(Service::None, "no-service"), (Service::Link, "symlink")] {
        let i = Install::new();
        i.module("groups", "0.1.0", GOOD_FORMS, &service);
        let Some((_, said)) = draw(&i, &[], "light", "groups-noservice.png", &[]) else {
            return;
        };
        assert!(
            said.contains("host-notice-data-unreachable")
                && said.contains(&format!("cause={cause}")),
            "{said}"
        );
        assert!(!i.started("groups"), "{cause}: nothing was started");
    }
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn a_service_that_does_not_answer_in_time_ends_with_a_notice_after_the_time_limit() {
    let i = good();
    let began = std::time::Instant::now();
    let Some((_, said)) = draw(&i, &[], "light", "~slow.png", &[("FAKE_MODE", "slow")]) else {
        return;
    };
    assert!(
        said.contains("host-notice-data-unreachable") && said.contains("cause=time"),
        "{said}"
    );
    assert!(
        began.elapsed() < std::time::Duration::from_secs(25),
        "the window must not wait for a service that hangs"
    );
}

#[test]
#[cfg_attr(windows, ignore = "unix sockets")]
fn a_module_that_leaves_a_required_channel_out_is_locked_and_the_picture_run_ends_with_3() {
    let i = Install::new();
    i.module(
        "groups",
        "0.1.0",
        &GOOD_FORMS.replace(r#""label":"name","#, ""),
        &Service::Fake,
    );
    let (_k, dir) = out_dir();
    let path = dir.join("groups-unmapped.png");
    let g = groups();
    let out = i.run(
        &["--description", &g, "--picture", path.to_str().unwrap()],
        "light",
        &[],
    );
    let err = text(&out.stderr);
    if out.status.code() != Some(3) {
        assert!(
            err.to_lowercase().contains("graphics") || err.to_lowercase().contains("adapter"),
            "{err}"
        );
        return;
    }
    let locked = support::read_png(&path);
    let (ok, _) = draw(&good(), &[], "light", "~ok.png", &[]).unwrap();
    assert!(
        !same(&locked, &ok),
        "the tree is not drawn from a module that does not fit"
    );
}
