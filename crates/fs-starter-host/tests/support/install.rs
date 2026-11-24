//! A made-up installation for the tests of the sources: a folder with a copy of the host binary and, next to it, `modules/<name>/`
//! (the bundled modules: `catalog/commands.json`, `catalog/forms.json`, `service`), and a runtime folder of its own. The service is the
//! fake module of this crate (`fs-starter-fake-module`, provider `fake`). Dropping the installation stops the services it started.

#![allow(dead_code)] // each test file uses a part

use std::path::{Path, PathBuf};
use std::process::Output;

use tempfile::TempDir;

/// The tree and member forms of the groups module as `forms.json` (the module's field names; the same ones the fake service answers).
pub const GOOD_FORMS: &str = r#"[
 {"name":"tree","version":"1.0","type":"Group",
  "map":{"nodes":"Group","parent":"parent","label":"name","edge-kind":"mode","owner-mark":"mine"},
  "fields":{"parent":"reference","name":"text","mode":"choice","mine":"bool"}},
 {"name":"rows","version":"1.0","type":"Member",
  "map":{"items":"Member","label":"person","columns":["role","level"]},
  "fields":{"person":"reference","role":"reference","level":"text"}}
]"#;

/// What stands at `<module>/service`.
pub enum Service {
    /// A copy of the fake module (made-up groups data).
    Fake,
    /// No program file.
    None,
    /// A symbolic link to the fake module (refused).
    Link,
}

/// A symbolic link `file` to `target`: `symlink` on unix, `symlink_file` on Windows (the runner of the CI is an administrator, so it may create one).
#[cfg(unix)]
pub fn link(target: &Path, file: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, file)
}

#[cfg(windows)]
pub fn link(target: &Path, file: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, file)
}

/// The fake module service: the EXAMPLE `fs-starter-fake-module` of this crate (not a `[[bin]]`, so no package format ever sees it). `cargo test`
/// builds examples (without a target filter) into `<target>/<profile>/examples/`; the test binary sits in `deps/` (classic layout) or deeper in
/// `build/<package>/<hash>/` (split build dir), so the folders above it are searched. Missing (a run with `--test <name>` builds no examples)
/// is a loud failure, never a silently skipped service.
pub fn fake_module() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let file = format!("fs-starter-fake-module{}", std::env::consts::EXE_SUFFIX);
    exe.ancestors()
        .skip(1)
        .take(6)
        .map(|a| a.join("examples").join(&file))
        .find(|p| p.is_file())
        .unwrap_or_else(|| {
            panic!(
                "the fake module `{file}` is not built (searched above {}): run `cargo test -p fs-starter-host` (or `cargo build -p fs-starter-host --examples` first)",
                exe.display()
            )
        })
}

pub struct Install {
    pub dir: TempDir,
    pub runtime: TempDir,
}

/// `commands.json` of a module of this version (one command of its own name space).
pub fn commands(module: &str, version: &str) -> String {
    format!(
        r#"{{"module":"{module}","version":"{version}","contract":"{module}","scope":"own-contract","commands":[{{"id":"{module}.task.create","call":"x","capability":"task.create","access":"write","help":{{"topic":"add","text":{{"en":"e","de":"d"}}}}}}]}}"#
    )
}

pub fn groups() -> String {
    format!(
        "{}/../../tests/data/groups.toml",
        env!("CARGO_MANIFEST_DIR")
    )
}

impl Install {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(
            env!("CARGO_BIN_EXE_fs-starter"),
            fs_starter::program_file(dir.path(), "fs-starter"),
        )
        .unwrap();
        std::fs::create_dir(dir.path().join("modules")).unwrap();
        Self {
            dir,
            runtime: tempfile::tempdir().unwrap(),
        }
    }

    /// The folder of the module `name` below `modules/` (next to the binary).
    pub fn folder(&self, name: &str) -> PathBuf {
        self.dir.path().join("modules").join(name)
    }

    /// Add a bundled module: its set, its forms and its service.
    pub fn module(&self, name: &str, version: &str, forms: &str, service: &Service) -> &Self {
        let catalog = self.folder(name).join("catalog");
        std::fs::create_dir_all(&catalog).unwrap();
        std::fs::write(catalog.join("commands.json"), commands(name, version)).unwrap();
        std::fs::write(catalog.join("forms.json"), forms).unwrap();
        let fake = fake_module();
        let fake = fake.as_path();
        let file = fs_starter::program_file(&self.folder(name), fs_starter::SERVICE_STEM);
        match service {
            Service::Fake => drop(std::fs::copy(fake, &file).unwrap()),
            Service::None => {}
            Service::Link => link(fake, &file).unwrap(),
        }
        self
    }

    /// The copied host with `args`; `env` is set on top of a clean environment (no session, own HOME and runtime folder).
    pub fn run(&self, args: &[&str], scheme: &str, env: &[(&str, &str)]) -> Output {
        let home = tempfile::tempdir().unwrap();
        let mut c =
            std::process::Command::new(fs_starter::program_file(self.dir.path(), "fs-starter"));
        c.args(args)
            .env("LANG", "en_US.UTF-8")
            .env("FS_COLOR_SCHEME", scheme)
            .env("HOME", home.path())
            .env("XDG_RUNTIME_DIR", self.runtime.path())
            .env("XDG_DATA_HOME", home.path())
            .env("XDG_CONFIG_HOME", home.path())
            .env_remove("FS_RENDER_SCHEME")
            .env_remove("LC_ALL")
            .env_remove("LC_MESSAGES")
            .env_remove("FAKE_MODE");
        for (k, v) in env {
            c.env(k, v);
        }
        // a binary that was just copied can be "busy" while another test forks (the copy's writer is still open in the child)
        for _ in 0..50 {
            match c.output() {
                Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                other => return other.unwrap(),
            }
        }
        c.output().unwrap()
    }

    /// The sockets of the services this installation started.
    fn sockets(&self) -> Vec<PathBuf> {
        let Ok(list) = std::fs::read_dir(self.runtime.path().join("freesynergy")) else {
            return Vec::new();
        };
        list.filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "sock"))
            .collect()
    }

    /// Whether the service of `module` was started (its socket is there).
    pub fn started(&self, module: &str) -> bool {
        self.runtime
            .path()
            .join("freesynergy")
            .join(format!("{module}.sock"))
            .exists()
    }
}

impl Drop for Install {
    fn drop(&mut self) {
        let sockets = self.sockets();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        tokio::task::LocalSet::new().block_on(&runtime, async {
            for s in &sockets {
                let _ = stop(s).await;
            }
        });
    }
}

async fn stop(socket: &Path) -> std::io::Result<()> {
    fs_rpc::stop_server(socket).await
}
