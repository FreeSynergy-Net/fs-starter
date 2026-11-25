//! The end of a service that a one-time command started itself: the guard lives in [`super::HeldRecords`], so the service ends when
//! the records are dropped (after the hold of the session is closed), never earlier. A service that was up before is never ended.
//!
//! Pattern: RAII / Guard – the drop is the end of the service this run started.

use std::path::PathBuf;

use super::service::finish;

/// Ends the service on `socket` when dropped.
#[derive(Debug)]
pub(super) struct EndOnDrop {
    socket: PathBuf,
}

impl EndOnDrop {
    /// The guard for a service this run started on `socket`.
    pub(super) fn new(socket: PathBuf) -> Self {
        Self { socket }
    }
}

impl Drop for EndOnDrop {
    fn drop(&mut self) {
        let socket = self.socket.clone();
        // an own thread with an own runtime: a drop may happen inside a runtime of the caller, where `block_on` would panic
        let ended = std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            let started = std::cell::Cell::new(true);
            let local = tokio::task::LocalSet::new();
            local.block_on(&runtime, async {
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    finish(&socket, &started, true),
                )
                .await;
            });
        })
        .join();
        if ended.is_err() {
            eprintln!("fs-starter: end of the started service failed");
        }
    }
}
