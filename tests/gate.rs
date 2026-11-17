//! K28 of ENTWURF section 5: the standard gate with a program that does not exist.
#![cfg(target_os = "linux")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use fs_rpc::{Expected, SingleTimes};
use fs_starter::coded_stub::Param;
use fs_starter::{ServerGate, SingleGate, StarterCode};

/// K28: the server cannot start: `server{cause}` with a known cause, well inside the overall time.
#[test]
fn k28_gate_without_program() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let socket = tmp.path().join("demo.sock");
    let times = SingleTimes {
        overall: Duration::from_secs(10),
        ..SingleTimes::default()
    };
    let gate = SingleGate::<capnp::capability::Client>::new(
        PathBuf::from("/nonexistent/fs-demo"),
        Expected::new("0.0.1").with_program("demo"),
        times,
    );
    let began = Instant::now();
    let e = gate.ensure(&socket).expect_err("no server can start");
    assert!(
        began.elapsed() < Duration::from_secs(11),
        "took {:?}",
        began.elapsed()
    );
    assert_eq!(e.code(), StarterCode::Server);
    let cause = e
        .params()
        .find(|(k, _)| *k == "cause")
        .map(|(_, v)| v.clone());
    let Some(Param::Text(cause)) = cause else {
        panic!("no cause: {e:?}")
    };
    assert!(
        ["cannot-restart", "no-server"].contains(&cause.as_str()),
        "cause {cause}"
    );
}
