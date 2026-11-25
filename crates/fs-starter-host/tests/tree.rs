//! The element tree of the real description of fs-tasks, read through the host's window library: no window, no graphics adapter.
//!
//! Tests: example-based. The blueprint folder (`scaffold.toml`, `[frontend] layout = "forms/board.toml"`) gives the page `board`
//! of the frame `workbench` with the side list (`sidebar`) and the board (`board`) as blocks that are drawn; with the sample
//! records the board has three columns. A bare `forms/` folder with three descriptions is not one (the host will not pick).
//! Mutant: the host reads another file than the one the scaffold names (`day.toml`) -> the page is `day`, the test is red.

mod support;
use support::tasks;

use fs_forms_run::{Blocks, FileLink, FormRecords, LayoutLink, Node, Records, compose};
use fs_forms_testdata::{SAMPLE_PROVIDER, SampleModule, SourceSpec};
use fs_interfaces::module::{Actor, Caller, Context, ModuleCore, PageQuery};
use fs_types::{BasicValue, PersonArt, PersonRef, Reference};

/// The wish of the sample board: five cards in three columns (`[sample]`, nothing else; a copy of the fs-forms-run fixture).
const WISH: &str = include_str!("data/board-parity.toml");

fn block_of<'a>(nodes: &'a [Node], place: &str) -> Option<&'a str> {
    nodes.iter().find_map(|n| match n {
        Node::Block {
            place: p, block, ..
        } if p == place => Some(block.as_str()),
        _ => None,
    })
}

#[test]
fn the_blueprint_of_a_program_gives_the_board_page_with_a_side_list_and_a_board() {
    let view = FileLink::new(tasks(), "en").resolve("", "").unwrap();
    let comp = compose(&view, &Blocks);
    let board: Vec<_> = comp.screens.iter().filter(|s| s.page == "board").collect();
    assert_eq!(board.len(), 1, "one page `board`: {:?}", comp.screens);
    assert_eq!(board[0].frame, "workbench");
    assert_eq!(block_of(&board[0].nodes, "side"), Some("sidebar"));
    assert_eq!(block_of(&board[0].nodes, "main"), Some("board"));
    // the list stands before the board in the tab order
    let at = |place: &str| {
        board[0]
            .nodes
            .iter()
            .position(|n| matches!(n, Node::Block { place: p, .. } if p == place))
    };
    assert!(at("side") < at("main"), "{:?}", board[0].nodes);
}

#[test]
fn the_sample_records_make_a_board_with_three_columns() {
    // the board is a form of the module (cards in columns): the columns are the distinct `column` values of its rows; the test data are
    // made from the wish by `fs-forms-testdata` and read through the module, as the host reads a module (no card or column is named)
    let set = SourceSpec::from_wish_text(WISH)
        .expect("the wish reads")
        .build()
        .expect("the wish builds");
    let module = SampleModule::new(&set).expect("the module");
    let slice = set
        .slices
        .iter()
        .zip(&set.declared)
        .find(|(_, d)| d.form.name == "cards-in-columns")
        .map(|(s, _)| s)
        .expect("the sample has the board form");
    let person = PersonRef {
        id: Reference::parse("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01").unwrap(),
        art: PersonArt::Person,
        abzug: None,
    };
    let caller = Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    let cx = Context::from_connection(caller, Actor::Person, person);
    let mut query = PageQuery::default();
    query.form.clone_from(&slice.form);
    query.limit = 500;
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let page = rt.block_on(module.page(&cx, query)).expect("page");
    let mut names: Vec<&str> = Vec::new();
    for column in page
        .records
        .iter()
        .flat_map(|r| r.values.iter().filter(|v| v.path == "column"))
    {
        if !names.contains(&column.text.as_str()) {
            names.push(&column.text);
        }
    }
    assert_eq!(page.records.len(), 5, "five cards");
    assert_eq!(names.len(), 3, "three columns: {names:?}");
    // sample data says so: a source with the provider of the test data is shown as such
    let records = FormRecords {
        provider: SAMPLE_PROVIDER.to_owned(),
        ..FormRecords::default()
    };
    assert!(records.is_sample(), "sample data says so");
}

#[test]
fn a_folder_of_the_description_files_alone_is_not_the_blueprint() {
    let forms = format!("{}/forms", tasks());
    // three descriptions, none asked for by name: the host does not pick one
    let err = FileLink::new(forms, "en").resolve("", "").unwrap_err();
    assert_eq!(err.code(), "layout-missing", "{err}");
    // asked by name, one is found
    let day = FileLink::new(format!("{}/forms", tasks()), "en")
        .resolve("", "day")
        .unwrap();
    assert_eq!(day.frontend, "day");
}
