//! The element tree of the real description of fs-tasks, read through the host's window library: no window, no graphics adapter.
//!
//! Tests: example-based. The blueprint folder (`scaffold.toml`, `[frontend] layout = "forms/board.toml"`) gives the page `board`
//! of the frame `workbench` with the side list (`sidebar`) and the board (`board`) as blocks that are drawn; with the sample
//! records the board has three columns. A bare `forms/` folder with three descriptions is not one (the host will not pick).
//! Mutant: the host reads another file than the one the scaffold names (`day.toml`) -> the page is `day`, the test is red.

mod support;
use support::tasks;

use fs_forms_run::{Blocks, FileLink, LayoutLink, Node, Records, SampleBoard, compose};

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
    // the board is a form of the module (cards in columns): the columns are the distinct `column-label` values of its rows
    let forms = SampleBoard::default().forms();
    let board = forms
        .iter()
        .find(|f| f.declared.form.name == "cards-in-columns")
        .expect("the sample has the board form");
    let mut names: Vec<&str> = Vec::new();
    for label in board.rows.iter().filter_map(|r| r.get("column-label")) {
        if !names.contains(&label) {
            names.push(label);
        }
    }
    assert_eq!(names, ["To do", "Doing", "Done"]);
    assert!(SampleBoard::default().is_sample(), "sample data says so");
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
