//! The real planning service (fs-modules-planning, one fixed commit) in this process behind a real Unix socket in a temporary folder,
//! with a board `work` and three cards, and the fixture of the suite for it: forms and commands are the REAL catalog files of that commit
//! (`tests/data/planning-main-7db66a33/catalog/`, a copy, never edited).
//!
//! Pattern: Template Method (the suite's rules; this is the fixture) + Fixture (a service in a temporary folder).

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs_interfaces::formsource_capnp::form_source;
use fs_interfaces::tasks_capnp::tasks;
use fs_module::{DeclaredCommand, DeclaredForm, FormSourceFixture, Sample, Verify};
use fs_modules_planning::Planning;
use fs_objects::{Store, TursoStore};
use fs_tasks_core::{Task, TaskRepo};
use tokio::sync::oneshot;

pub const FORM: &str = "cards-in-columns@1.4.0";
const FORMS: &str =
    include_str!("../../../../tests/data/planning-main-7db66a33/catalog/forms.json");
const COMMANDS: &str =
    include_str!("../../../../tests/data/planning-main-7db66a33/catalog/commands.json");

/// The service on a socket in `dir` (data and socket there); ends when the sender is dropped or used.
pub async fn serve(dir: &Path) -> (PathBuf, oneshot::Sender<()>) {
    // the store has an owner (fs-objects 0.0.11), as the service opens it
    let owner = fs_objects::testing::owner();
    let options = fs_objects::OpenOptions::default().with_owner(owner.clone());
    let store = TursoStore::open_with(dir.join("objects.db"), options)
        .await
        .unwrap();
    let store: Arc<dyn Store> = Arc::new(store);
    let repo = Arc::new(TaskRepo::new(
        store,
        "https://local.invalid/users/owner",
        owner,
    ));
    let socket = dir.join("planning.sock");
    let bound = fs_rpc::bind_socket(&socket).await.unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    tokio::task::spawn_local(async move {
        let _ = fs_rpc::serve_callers(bound, Planning::new(repo, |_| {}), async {
            let _ = stopped.await;
        })
        .await;
    });
    (socket, stop)
}

/// The board `work` with cards of these titles (standard columns, in `todo`); the uids of the cards in the order of the first page.
pub async fn board_with(
    socket: &Path,
    source: &form_source::Client,
    titles: &[&str],
) -> Vec<String> {
    let tasks = fs_rpc::connect::<tasks::Client>(socket).await.unwrap();
    let boards = tasks.client.boards_request().send().pipeline.get_boards();
    let mut create = boards.create_request();
    create.get().set_name("work");
    create.send().promise.await.unwrap();
    let items = tasks.client.items_request().send().pipeline.get_items();
    for title in titles {
        let mut add = items.create_request();
        add.get()
            .set_jscalendar(Task::new(*title).to_value().to_string().as_str());
        add.send().promise.await.unwrap();
    }
    let mut request = source.page_request();
    {
        let mut p = request.get();
        p.set_form("cards-in-columns@1.1");
        p.set_scope("work");
        p.set_limit(100);
    }
    let answer = request.send().promise.await.unwrap();
    let records = answer
        .get()
        .unwrap()
        .get_page()
        .unwrap()
        .get_records()
        .unwrap();
    records
        .iter()
        .map(|r| r.get_id().unwrap().to_string().unwrap())
        .collect()
}

/// The fixture of the suite for planning: the catalog files of the commit, valid calls on the cards `uids`.
pub struct Planning3 {
    pub client: form_source::Client,
    /// A second connection to the same service (another program: the rules L2-8 and L2-11).
    pub other: form_source::Client,
    pub uids: Vec<String>,
}

impl FormSourceFixture for Planning3 {
    fn client(&self) -> form_source::Client {
        self.client.clone()
    }
    fn level(&self) -> u32 {
        fs_modules_planning::LEVEL
    }
    fn second(&self) -> Option<form_source::Client> {
        Some(self.other.clone())
    }
    fn provider(&self) -> Option<&str> {
        Some("planning")
    }
    fn forms(&self) -> Vec<DeclaredForm> {
        let forms: serde_json::Value = serde_json::from_str(FORMS).unwrap();
        vec![DeclaredForm {
            key: FORM.into(),
            scope: "work".into(),
            fields: forms[0]["fields"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect(),
        }]
    }
    fn commands(&self) -> Vec<DeclaredCommand> {
        super::module::declared_commands(COMMANDS)
    }
    fn samples(&self) -> Vec<Sample> {
        let a = |k: &str, v: &str| (k.to_owned(), v.to_owned());
        let value = |path: &str, text: &str| Verify::Value {
            form: FORM.into(),
            record: None,
            path: path.into(),
            text: text.into(),
        };
        vec![
            Sample {
                command: "card.move".into(),
                record: self.uids[2].clone(),
                args: vec![
                    a("board", "work"),
                    a("column", "doing"),
                    a("after", ""),
                    a("before", ""),
                ],
                makes_record: true,
                verify: value("column", "doing"),
            },
            Sample {
                command: "task.create".into(),
                record: String::new(),
                args: vec![
                    a("title", "Made"),
                    a("board", "work"),
                    a("column", "todo"),
                    a("before", &self.uids[0]),
                ],
                makes_record: true,
                verify: value("title", "Made"),
            },
            Sample {
                command: "task.delete".into(),
                record: self.uids[1].clone(),
                args: vec![],
                makes_record: false,
                verify: Verify::Gone {
                    form: FORM.into(),
                    record: self.uids[1].clone(),
                },
            },
        ]
    }
    fn strict_scope(&self) -> bool {
        true
    }
}
