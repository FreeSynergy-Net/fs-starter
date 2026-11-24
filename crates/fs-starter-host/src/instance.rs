//! One window per program (ADR 0097, "single instance"): the first start of a program serves the interface `Instance`
//! (`schema/instance.capnp` of fs-interfaces) on a socket of its own; a second start of the same program does not open a second
//! window, it hands its words to the first and ends.
//!
//! The endpoint is `default_socket("<name>.window")` of fs-rpc: a `.` cannot be in a program name, so it never clashes with the
//! `<name>.sock` of a server or with another program. The name goes through `ProgramName::parse` first. Without
//! `XDG_RUNTIME_DIR` there is no endpoint (and no fallback below `/tmp`): the program starts anyway, with one line
//! `single-instance-off`. Who wins is decided by the lock of `fs_rpc::bind_socket`, never by a second window.
//!
//! Left out for now: bringing the window to the front, opening the handed-over path in the running window (a start is only
//! received and noted as `instance-start-received`), macOS, killing a hung instance. Further windows (`--new-instance`) are in `slots.rs`.
//!
//! Pattern: Active object (the serving thread owns the socket) + RAII (the [`Guard`] frees thread, socket and lock).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use fs_interfaces::instance_capnp::instance;
use fs_starter::ProgramName;
use tokio::sync::oneshot;

/// The code of the line when there is no endpoint (no `XDG_RUNTIME_DIR`).
pub const OFF: &str = "single-instance-off";
/// The code when the name of the program cannot be an endpoint.
pub const NAME_INVALID: &str = "instance-name-invalid";
/// The code of the line a running window prints for a start that was handed to it.
pub const START_RECEIVED: &str = "instance-start-received";
/// The code of the line a second start prints before it ends.
pub const HANDED_OVER: &str = "instance-handed-over";

/// How many handed-over starts wait for the window; more are refused (`accepted = false`).
const QUEUE: usize = 16;
/// At most this many words, each at most this many bytes, are taken (the read limits of fs-rpc cap the message itself).
const MAX_WORDS: usize = 64;
const MAX_WORD_BYTES: usize = 4096;
/// The pause between two tries to reach a first instance that holds the lock but is not bound yet.
const RETRY: Duration = Duration::from_millis(50);

/// The two waits of a second start.
#[derive(Clone, Copy, Debug)]
pub struct InstanceTimes {
    /// How long to retry reaching a first instance that holds the lock but is not yet bound.
    pub connect: Duration,
    /// How long to wait for the answer of a first instance that is reached.
    pub answer: Duration,
}

impl Default for InstanceTimes {
    fn default() -> Self {
        Self {
            connect: Duration::from_secs(2),
            answer: Duration::from_secs(2),
        }
    }
}

/// Decides whether the first instance takes the words (the normal word parser in the host; a test can say otherwise).
pub type Check = fn(&[String]) -> bool;

/// The words are taken when the normal word reader of the window host reads them as the opening of a window.
#[must_use]
pub fn words_are_a_start(words: &[String]) -> bool {
    matches!(
        fs_forms_run::parse(words),
        Ok(fs_forms_run::Command::Window(_))
    )
}

/// The endpoint of a program: where its first window listens. `None` without `XDG_RUNTIME_DIR`.
#[must_use]
pub fn endpoint(program: &ProgramName) -> Option<PathBuf> {
    fs_rpc::default_socket(&format!("{program}.window"))
}

/// The endpoint of the program called `name`: the name is checked first (`ProgramName::parse`), no path is built for a bad one.
///
/// # Errors
/// The code `instance-name-invalid` (with the name).
pub fn endpoint_for(name: &str) -> Result<Option<PathBuf>, String> {
    let program = ProgramName::parse(name).ok_or_else(|| format!("{NAME_INVALID}: {name}"))?;
    Ok(endpoint(&program))
}

/// What a start learns.
pub enum Claim {
    /// This is the first start: it holds the place until the guard is dropped.
    First(Guard),
    /// Another start of the program runs: the words are handed to it, this start ends.
    HandedOver,
    /// No endpoint: the program starts without the single-instance rule (the code to say).
    Off(&'static str),
}

/// The place of the first instance: the thread that serves `Instance`, the socket and its lock. Dropping it ends all three.
pub struct Guard {
    /// The starts handed over by later starts (the words after the program name), oldest first.
    pub starts: Receiver<Vec<String>>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The server side: what a handed-over start does in the running window (so far: it is noted and queued).
struct Receiving {
    check: Check,
    starts: SyncSender<Vec<String>>,
}

#[allow(clippy::unused_async_trait_impl)] // the generated trait wants an async method
impl instance::Server for Receiving {
    async fn open(
        self: capnp::capability::Rc<Self>,
        params: instance::OpenParams,
        mut results: instance::OpenResults,
    ) -> Result<(), capnp::Error> {
        let reader = params.get()?.get_words()?;
        let mut words = Vec::new();
        let mut fits = (reader.len() as usize) <= MAX_WORDS;
        if fits {
            for word in reader {
                let word = word?.to_str()?;
                fits &= word.len() <= MAX_WORD_BYTES;
                words.push(word.to_owned());
            }
        }
        // nothing is executed: the words are read by the normal parser, and only a start of a window is queued
        let accepted = fits && (self.check)(&words) && self.starts.try_send(words).is_ok();
        if accepted {
            eprintln!("fs-starter: {START_RECEIVED}");
        }
        results.get().set_accepted(accepted);
        Ok(())
    }
}

fn serve_thread(
    path: &Path,
    check: Check,
    starts: SyncSender<Vec<String>>,
    stop: oneshot::Receiver<()>,
    ready: &SyncSender<io::Result<()>>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    tokio::task::LocalSet::new().block_on(&runtime, async {
        let bound = match fs_rpc::bind_socket(path).await {
            Ok(b) => b,
            Err(e) => {
                let _ = ready.send(Err(e));
                return;
            }
        };
        let _ = ready.send(Ok(()));
        let root: instance::Client = capnp_rpc::new_client(Receiving { check, starts });
        let _ = fs_rpc::serve(bound, root.client, async {
            let _ = stop.await;
        })
        .await;
    });
}

/// Take the place of the first instance of the program on `endpoint`, or hand `words` to the one that has it.
///
/// # Errors
/// A line with its code: `instance-endpoint-unusable` (the folder is not private or the system refuses),
/// `instance-unreachable` (the lock is held but nobody answers within `times.connect`), `instance-not-answering` (the first
/// instance is reached but does not answer within `times.answer`: it is never killed), `instance-words-refused` (it does not
/// take these words). In none of these cases a second window is opened.
pub fn claim(
    endpoint: Option<&Path>,
    words: &[String],
    times: InstanceTimes,
) -> Result<Claim, String> {
    claim_with(endpoint, words, times, words_are_a_start)
}

/// [`claim`] with another rule for the words the first instance takes.
///
/// # Errors
/// As for [`claim`].
pub fn claim_with(
    endpoint: Option<&Path>,
    words: &[String],
    times: InstanceTimes,
    check: Check,
) -> Result<Claim, String> {
    let Some(path) = endpoint else {
        return Ok(Claim::Off(OFF));
    };
    match claim_free(path, check)? {
        Some(guard) => Ok(Claim::First(guard)),
        None => hand_over(path, words, times).map(|()| Claim::HandedOver),
    }
}

/// Take the place on `path` if it is free: `Some(guard)`; `None` when someone holds it (nothing is handed over, no window is
/// asked). The lock of `fs_rpc::bind_socket` decides.
///
/// # Errors
/// `instance-endpoint-unusable` (the folder is not private or the system refuses).
pub fn claim_free(path: &Path, check: Check) -> Result<Option<Guard>, String> {
    let (stop_tx, stop_rx) = oneshot::channel();
    let (starts_tx, starts_rx) = sync_channel(QUEUE);
    let (ready_tx, ready_rx) = sync_channel(1);
    let own = path.to_path_buf();
    let thread = std::thread::Builder::new()
        .name("instance".into())
        .spawn(move || serve_thread(&own, check, starts_tx, stop_rx, &ready_tx))
        .map_err(|e| format!("instance-endpoint-unusable: {e}"))?;
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(Some(Guard {
            starts: starts_rx,
            stop: Some(stop_tx),
            thread: Some(thread),
        })),
        Ok(Err(e)) if e.kind() == io::ErrorKind::AddrInUse => {
            let _ = thread.join();
            Ok(None)
        }
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(format!(
                "instance-endpoint-unusable: {}: {e}",
                path.display()
            ))
        }
        Err(_) => Err("instance-endpoint-unusable: the serving thread ended".to_owned()),
    }
}

/// Whether a failed connect may be a first instance that is not bound yet (or a socket that is not answered yet).
fn not_there_yet(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
    )
}

/// Call `open(words)` on the first instance.
fn hand_over(path: &Path, words: &[String], times: InstanceTimes) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("instance-unreachable: {e}"))?;
    tokio::task::LocalSet::new().block_on(&runtime, async {
        let deadline = Instant::now() + times.connect;
        let connection = loop {
            match fs_rpc::connect::<instance::Client>(path).await {
                Ok(c) => break c,
                Err(e) if not_there_yet(&e) && Instant::now() < deadline => {
                    tokio::time::sleep(RETRY).await;
                }
                Err(e) => return Err(format!("instance-unreachable: {}: {e}", path.display())),
            }
        };
        let mut request = connection.client.open_request();
        let mut list = request.get().init_words(
            u32::try_from(words.len()).map_err(|_| "instance-words-refused: too many")?,
        );
        for (i, word) in (0u32..).zip(words) {
            list.set(i, word);
        }
        let answer = tokio::time::timeout(times.answer, request.send().promise)
            .await
            .map_err(|_| "instance-not-answering".to_owned())?
            .map_err(|e| format!("instance-unreachable: {e}"))?;
        let accepted = answer
            .get()
            .map(instance::open_results::Reader::get_accepted)
            .map_err(|e| format!("instance-unreachable: {e}"))?;
        if accepted {
            Ok(())
        } else {
            Err("instance-words-refused: the running window does not take these words".to_owned())
        }
    })
}
