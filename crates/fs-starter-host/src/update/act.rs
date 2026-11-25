//! The real [`Act`]: the heartbeat `save` and the end of a module service through fs-rpc, and the exec of the own file.
//!
//! Pattern: Adapter (fs-rpc's `ask_save` and `stop_server`, the system's `exec`, to the `Act` strategy). Nothing is ended before its
//! save answered; a save that failed leaves the service running.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use fs_rpc::{Saved, ask_save, stop_server};

use super::marker::Marker;
use super::watch::Act;

/// How long the service may take to save.
const SAVE_LIMIT: Duration = Duration::from_secs(10);

/// The act of the host.
pub struct RpcAct {
    /// The own program file as it was at the start (never `current_exe` later: after a replacement it can read `... (deleted)`).
    pub own_exe: PathBuf,
    /// The words the host was started with (without the program).
    pub args: Vec<OsString>,
    /// The mark the next start reads.
    pub marker: Option<Marker>,
    /// The version that runs.
    pub version: String,
}

impl Act for RpcAct {
    fn end_module(&self, module: &str) -> Result<Option<Saved>, String> {
        let socket = fs_rpc::default_socket(module).ok_or("no-socket")?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "runtime".to_owned())?;
        let local = tokio::task::LocalSet::new();
        local.block_on(&runtime, async {
            match ask_save(&socket, SAVE_LIMIT).await {
                Ok(Saved::Failed) => Err("save-failed".to_owned()),
                Ok(saved) => stop_server(&socket)
                    .await
                    .map(|()| Some(saved))
                    .map_err(|e| format!("end-{:?}", e.kind())),
                Err(e) if is_not_running(&e) => Ok(None),
                Err(e) => Err(format!("save-{:?}", e.kind())),
            }
        })
    }

    #[cfg(unix)]
    fn restart_own(&self) -> Result<(), String> {
        use super::marker::{Mark, State};
        use std::os::unix::process::CommandExt;
        if let Some(m) = &self.marker {
            let mut mark = Mark::running(&self.version, true);
            mark.state = State::Restarted;
            m.write(&mark)
                .map_err(|e| format!("marker-{:?}", e.kind()))?;
        }
        // only returns on failure
        let e = std::process::Command::new(&self.own_exe)
            .args(&self.args)
            .exec();
        Err(format!("exec-{:?}", e.kind()))
    }

    #[cfg(not(unix))]
    fn restart_own(&self) -> Result<(), String> {
        Err("restart-not-built-on-this-system".to_owned())
    }
}

fn is_not_running(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
    )
}
