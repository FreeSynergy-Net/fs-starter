//! H-3: a window HOLDS its module. The records `connect` gives hold one connection to the service (`fs_rpc::hold`); the module ends
//! by itself after its idle time, so it must live while the records live and end after they dropped. The fake ends after 300 ms
//! without a connection (`FAKE_IDLE_MS`). The lines the hold says are given to the test (`connect_saying`), the same ones stderr gets.
//!
//! Mutant: no hold in `connect` (`end_started` branch always) = the fake ends during the wait, the first assert is red.
#![cfg(unix)]

mod support;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use support::inproc::setup;
use support::install::{GOOD_FORMS, commands};

const IDLE: Duration = Duration::from_millis(300);

fn line(rx: &mpsc::Receiver<String>) -> String {
    rx.recv_timeout(Duration::from_secs(5))
        .expect("a hold line")
}

#[test]
fn the_module_lives_while_the_records_are_held_and_ends_after_they_dropped() {
    let s = setup("groups", GOOD_FORMS, &commands("groups", "0.1.0"), 300);
    let (tx, rx) = mpsc::channel();
    let records = s
        .sources
        .connect_saying("groups@>=0.1.0, <0.2.0", move |l| tx.send(l).unwrap())
        .expect("read and hold");
    assert_eq!(line(&rx), "fs-starter: hold module=groups attached");
    // three times the idle time with nobody asking: only the hold keeps the module alive
    std::thread::sleep(IDLE * 3);
    assert!(s.socket.exists(), "the module ended while it was held");
    assert!(!records.forms().is_empty(), "the records are the read ones");
    drop(records);
    let dropped = Instant::now();
    assert_eq!(line(&rx), "fs-starter: hold module=groups detached");
    while s.socket.exists() {
        assert!(
            dropped.elapsed() < Duration::from_secs(5),
            "the module did not end after the hold dropped"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(rx.try_recv().is_err(), "no more lines after `detached`");
}
