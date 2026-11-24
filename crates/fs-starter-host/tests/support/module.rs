//! A module service as a process and the suite `fs_module::conformance` (`module_v1`) against it: shared by the contract tests of the
//! host. Unix only (sockets, heartbeat of fs-rpc).
//!
//! Pattern: Template Method (the rules are the suite's; the fixtures here only say what the service declares and how to call it).

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use fs_interfaces::formsource_capnp::form_source;
use fs_module::{
    CommandKind, DeclaredCommand, DeclaredForm, FakeSource, FormSourceFixture, LaneCase, Ordered,
    Sample, ValueField, ValueKind,
};

/// A child process that is ended at the end of its test, also when the test fails.
pub struct Running(pub Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The fake module as a process (EXAMPLE `fs-starter-fake-module`) with its own runtime folder; returns the socket it serves on.
pub fn start_fake(runtime: &Path, mode: &str) -> (Running, PathBuf) {
    let child = Command::new(super::install::fake_module())
        .arg("serve")
        .env_clear()
        .env("FAKE_MODULE", "cards")
        .env("FAKE_MODE", mode)
        .env("XDG_RUNTIME_DIR", runtime)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    (
        Running(child),
        runtime.join("freesynergy").join("cards.sock"),
    )
}

/// The `FormSource` of the service on `socket` over the real RPC connection (the one of the host); waits for the socket. The connection must be kept: dropping it ends it.
pub async fn connect(socket: &Path) -> fs_rpc::Connection<form_source::Client> {
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        match fs_rpc::connect::<form_source::Client>(socket).await {
            Ok(c) => return c,
            Err(e) => {
                assert!(
                    Instant::now() < until,
                    "no service on {}: {e}",
                    socket.display()
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}

/// The fixture of the fake: what `FakeSource` declares, over the connection to the process.
pub struct FakeFixture(pub form_source::Client, pub Option<form_source::Client>);

impl FormSourceFixture for FakeFixture {
    fn client(&self) -> form_source::Client {
        self.0.clone()
    }
    fn level(&self) -> u32 {
        2
    }
    fn second(&self) -> Option<form_source::Client> {
        self.1.clone()
    }
    fn provider(&self) -> Option<&str> {
        Some("fake")
    }
    fn forms(&self) -> Vec<DeclaredForm> {
        FakeSource::forms()
    }
    fn commands(&self) -> Vec<DeclaredCommand> {
        FakeSource::commands()
    }
    fn samples(&self) -> Vec<Sample> {
        FakeSource::samples()
    }
    fn strict_scope(&self) -> bool {
        true
    }
    fn forbidden(&self) -> Option<(String, String)> {
        Some(("card.move".into(), "c3".into()))
    }
    fn forbidden_set(&self) -> Option<String> {
        Some("card.clear".into())
    }
    fn time_fields(&self) -> Vec<(String, String)> {
        vec![("cards-in-columns@1.0.0".into(), "due".into())]
    }
    fn ordered(&self) -> Option<Ordered> {
        Some(Ordered {
            command: "card.move".into(),
            form: "cards-in-columns@1.0.0".into(),
            record: "c1".into(),
            neighbour: "c3".into(),
            args: vec![("column".into(), "done".into())],
        })
    }
    fn value_fields(&self) -> Vec<ValueField> {
        [
            ("description", ValueKind::Description),
            ("color", ValueKind::Color),
            ("estimated", ValueKind::Estimated),
            ("column-limit", ValueKind::ColumnLimit),
            ("priority", ValueKind::Priority),
        ]
        .into_iter()
        .map(|(p, kind)| ValueField {
            form: "cards-in-columns@1.0.0".into(),
            path: p.into(),
            kind,
        })
        .collect()
    }
    fn create_without_lane(&self) -> Option<LaneCase> {
        Some(LaneCase {
            command: "card.create".into(),
            args: vec![
                ("title".into(), "No lane".into()),
                ("column".into(), "todo".into()),
            ],
            accepted: true,
        })
    }
}

/// One `invoke` over the connection: the whole answer, or the error of the call.
pub async fn invoke(
    client: &form_source::Client,
    command: &str,
    record: &str,
    args: &[(&str, &str)],
) -> capnp::Result<(bool, String, String)> {
    let mut request = client.invoke_request();
    {
        let mut p = request.get();
        p.set_command(command);
        p.set_record(record);
        let mut list = p.init_args(u32::try_from(args.len()).unwrap());
        for (i, (k, v)) in (0u32..).zip(args) {
            let mut a = list.reborrow().get(i);
            a.set_path(k);
            a.set_text(v);
        }
    }
    let answer = request.send().promise.await?;
    let a = answer.get()?;
    Ok((
        a.get_failed(),
        a.get_problem()?.get_type()?.to_string()?,
        a.get_result()?.get_record()?.to_string()?,
    ))
}

/// A command of a module's `commands.json` as the suite wants it (`capability` = the contract key; `task.create` works on the set).
pub fn declared_commands(file: &str) -> Vec<DeclaredCommand> {
    let file: serde_json::Value = serde_json::from_str(file).unwrap();
    file["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let key = c["capability"].as_str().unwrap().to_owned();
            DeclaredCommand {
                kind: if key == "task.create" {
                    CommandKind::Set
                } else {
                    CommandKind::Record
                },
                params: c["params"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| fs_module::Param {
                        name: p["path"].as_str().unwrap().to_owned(),
                        required: p["required"] == true,
                    })
                    .collect(),
                key,
            }
        })
        .collect()
}
