//! Making sure the program's server runs before a frontend opens (ADR 0094 item 4 a).
//!
//! Pattern: Strategy (`ServerGate`) + Adapter (`SingleGate` over fs-rpc `ensure_single`, rpc-0003 `single/ensure.rs:92`, and
//! `CommandStarter`, `starter.rs:26`). Extends what activities does with `Hello::fetch_expected` (`starter.rs:117`) by the
//! take-over of a hanging server of this program (fs-rpc 0.0.3) and makes the result a code.

use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use capnp::capability::FromClientHook;
use fs_rpc::{CommandStarter, Expected, Param, RpcError, SingleTimes, ensure_single};
use fs_types::coded::Coded;

use crate::codes::{StarterCode, keys};

/// A server of this program that was saved and ended so the new one could start (fs-rpc `TakenOver`): told as a hint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TakenOverNote {
    /// Its process.
    pub pid: u32,
    /// `saved`, `nothing`, `failed` (fs-rpc `Saved::token`).
    pub saved: &'static str,
    /// `gone`, `sigterm`, `sigkill`.
    pub how: &'static str,
}

/// The server runs and fits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Up {
    /// A hanging server was replaced on the way.
    pub taken_over: Option<TakenOverNote>,
}

/// Makes sure the server on a socket runs and fits.
pub trait ServerGate {
    /// Starts, replaces or takes over the server as needed; returns when it answers.
    ///
    /// # Errors
    /// `server` with `cause` (fs-rpc code name: `server-unresponsive`, `server-busy`, `contract-mismatch`, ...) and the
    /// fs-rpc parameters after it.
    fn ensure(&self, socket: &Path) -> Result<Up, Coded<StarterCode>>;
}

/// `server` with the fs-rpc code as `cause`, then the fs-rpc parameters in their order (keys unchanged). A key that appears
/// twice is kept twice; `Code::param` finds the first (fs-conformance review finding K-b).
#[must_use]
pub fn server_code(e: &RpcError) -> Coded<StarterCode> {
    let mut c = Coded::new(StarterCode::Server).with_text(keys::CAUSE, e.code().as_str());
    for (k, v) in e.params() {
        c = match v {
            Param::Text(t) => c.with_text(k, t.as_str()),
            Param::Number(n) => c.with_number(k, *n),
        };
    }
    c
}

/// The standard gate: `ensure_single::<C>` with `<program> serve --socket S` as the server's start. `C` is the program's
/// contract client (the starter itself does not call it). Runs its own single-thread runtime (capnp-rpc needs a `LocalSet`),
/// so `run` stays synchronous; the connection is dropped at once (the frontend makes its own).
pub struct SingleGate<C> {
    program: PathBuf,
    expected: Expected,
    times: SingleTimes,
    client: PhantomData<fn() -> C>,
}

impl<C> SingleGate<C> {
    /// `program` is the program's own file (`std::env::current_exe()`), `expected` its name and version
    /// (`Expected::new(version).with_program(name)`), `times` the limits (default: the plan's).
    #[must_use]
    pub fn new(program: PathBuf, expected: Expected, times: SingleTimes) -> Self {
        Self {
            program,
            expected,
            times,
            client: PhantomData,
        }
    }
}

impl<C: FromClientHook> ServerGate for SingleGate<C> {
    fn ensure(&self, socket: &Path) -> Result<Up, Coded<StarterCode>> {
        let io = |e: &std::io::Error| server_code(&RpcError::from(e));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| io(&e))?;
        let words: [std::ffi::OsString; 3] = [
            "serve".into(),
            "--socket".into(),
            socket.as_os_str().to_owned(),
        ];
        let starter = CommandStarter::new(&self.program, words);
        let local = tokio::task::LocalSet::new();
        // The connection is dropped inside the `LocalSet`, as activities `hello.rs` does.
        let single = local.block_on(&runtime, async {
            let s = ensure_single::<C>(socket, &self.expected, &starter, &self.times, None).await?;
            Ok::<_, fs_rpc::SingleError>(s.taken_over.map(|t| TakenOverNote {
                pid: t.pid,
                saved: t.saved.token(),
                how: t.how,
            }))
        });
        match single {
            Ok(taken_over) => Ok(Up { taken_over }),
            Err(e) => Err(server_code(&RpcError::from(&e))),
        }
    }
}

/// The taken-over hint as a code (`server-taken-over`: `pid`, `saved`, `how`).
pub(crate) fn taken_over_code(t: &TakenOverNote) -> Coded<StarterCode> {
    Coded::new(StarterCode::ServerTakenOver)
        .with_number(keys::PID, u64::from(t.pid))
        .with_text(keys::SAVED, t.saved)
        .with_text(keys::HOW, t.how)
}
