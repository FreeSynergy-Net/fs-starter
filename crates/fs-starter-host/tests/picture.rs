//! The window of the real description of fs-tasks (copy in `tests/data/fs-tasks`), drawn through the host binary without a
//! window (`--picture`) and read back as pixels. Where the machine has no graphics adapter a test says so and ends green.
//!
//! Tests: pixel tests. (1) `--description <blueprint> --sample-data` draws the board: three columns with cards, not one flat colour,
//! dark darker than light, sample data changes the picture; (2) the side list stands beside the board, not above it; (3) a missing
//! description draws an error window (`host-error-layout-missing`), different from the board window, in light and dark.
//! Mutants: the host ignores `--description` (all pictures equal); the error window is blank; dark is drawn as light.
//! Set `FS_PICTURE_OUT=<dir>` to keep the pictures (`starter-host-board-light.png`, `-dark.png`, `starter-host-error-light.png`).

mod support;
use support::{Picture, draw, draw_code, host, mean, out_dir, tasks, text, uniform};

fn colours(p: &Picture) -> usize {
    p.rgb
        .chunks(3)
        .map(|c| (c[0], c[1], c[2]))
        .collect::<std::collections::HashSet<_>>()
        .len()
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_board_of_a_description_is_drawn_light_and_dark_with_sample_cards() {
    let (_keep, dir) = out_dir();
    let tasks = tasks();
    let args = ["--description", tasks.as_str(), "--sample-data"];
    let Some(light) = draw(&args, "light", &dir.join("starter-host-board-light.png")) else {
        return;
    };
    let dark = draw(&args, "dark", &dir.join("starter-host-board-dark.png")).unwrap();
    let empty = draw(
        &["--description", &tasks],
        "light",
        &dir.join("starter-host-board-empty-light.png"),
    )
    .unwrap();
    assert_eq!((light.w, light.h), (1000, 760));
    assert!(
        light.rgb.iter().any(|b| *b != light.rgb[0]),
        "not one flat colour"
    );
    assert_ne!(light.rgb, dark.rgb);
    assert!(
        mean(&dark) + 20.0 < mean(&light),
        "dark {} is darker than light {}",
        mean(&dark),
        mean(&light)
    );
    assert_ne!(light.rgb, empty.rgb, "sample records change the window");
    assert!(
        colours(&light) > colours(&empty) + 5,
        "cards are drawn: {} vs {}",
        colours(&light),
        colours(&empty)
    );
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_side_list_stands_beside_the_board_in_three_columns_light_and_dark() {
    let (_keep, dir) = out_dir();
    let tasks = tasks();
    for scheme in ["light", "dark"] {
        let name = format!("starter-host-board-side-{scheme}.png");
        let Some(p) = draw(
            &["--description", &tasks, "--sample-data"],
            scheme,
            &dir.join(name),
        ) else {
            return;
        };
        // sample records make one page: the shell shows no navigation, the list is the strip at the left (x 0..220)
        assert!(
            !uniform(&p, 40..100, 70..180),
            "{scheme}: the list is drawn in the strip"
        );
        assert!(
            uniform(&p, 30..210, 300..700),
            "{scheme}: the strip is empty below the list"
        );
        // the board starts right of the strip at the top of the main area, and is as tall as it
        assert!(
            !uniform(&p, 240..480, 125..200),
            "{scheme}: the board begins beside the list"
        );
        assert!(
            !uniform(&p, 260..470, 700..712),
            "{scheme}: the board fills the main area"
        );
        // three columns: the cards of the sample data stand in three separate bands across the main area (x 240..1000)
        let band = |x0: u32, x1: u32| !uniform(&p, x0..x1, 150..400);
        assert!(
            band(240, 440) && band(440, 720) && band(720, 990),
            "{scheme}: three columns are drawn"
        );
    }
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn in_a_wide_window_the_third_column_is_whole_and_the_gaps_between_columns_are_empty() {
    let (_keep, dir) = out_dir();
    let tasks = tasks();
    let args = [
        "--description",
        tasks.as_str(),
        "--sample-data",
        "--size",
        "1300x760",
    ];
    let Some(p) = draw(
        &args,
        "light",
        &dir.join("starter-host-board-wide-light.png"),
    ) else {
        return;
    };
    assert_eq!((p.w, p.h), (1300, 760));
    // the three columns share the main area (x 245..581, 592..927, 938..1274), the rows of the first cards
    for (x0, x1) in [(247, 580), (595, 926), (941, 1273)] {
        assert!(!uniform(&p, x0..x1, 150..300), "a column at x {x0}..{x1}");
    }
    // between the columns (8 px) and right of the third: the board's own background, one colour
    assert!(uniform(&p, 585..589, 300..700), "the gap after column 1");
    assert!(uniform(&p, 931..935, 300..700), "the gap after column 2");
    assert!(
        uniform(&p, 1278..1298, 300..700),
        "nothing right of column 3"
    );
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_window_run_names_the_pending_parts_once_on_stderr() {
    let (_keep, dir) = out_dir();
    let tasks = tasks();
    let file = dir.join("starter-host-pending.png");
    let out = host(
        &["--description", &tasks, "--picture", file.to_str().unwrap()],
        "light",
    );
    let err = text(&out.stderr);
    if out.status.code() != Some(0) {
        assert!(err.to_lowercase().contains("adapter") || err.to_lowercase().contains("graphics"));
        return;
    }
    assert_eq!(err.matches("skeleton-part-pending").count(), 1, "{err}");
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn a_missing_description_is_an_error_window_not_a_blank_or_the_board() {
    let (_keep, dir) = out_dir();
    let tasks = tasks();
    let Some(board) = draw(
        &["--description", &tasks, "--sample-data"],
        "light",
        &dir.join("b-light.png"),
    ) else {
        return;
    };
    let gone = format!("{tasks}/no-such-description");
    let light = draw_code(
        &["--description", &gone],
        "light",
        &dir.join("starter-host-error-light.png"),
        3,
    )
    .unwrap();
    let dark = draw_code(
        &["--description", &gone],
        "dark",
        &dir.join("starter-host-error-dark.png"),
        3,
    )
    .unwrap();
    assert!(light.rgb.iter().any(|b| *b != light.rgb[0]), "not blank");
    assert_ne!(light.rgb, board.rgb, "not the board window");
    assert_ne!(light.rgb, dark.rgb);
    assert!(mean(&dark) + 20.0 < mean(&light));
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn a_picture_of_a_missing_description_is_written_and_the_run_ends_with_code_3() {
    let (_keep, dir) = out_dir();
    let file = dir.join("starter-host-error-code.png");
    let _ = std::fs::remove_file(&file);
    let gone = format!("{}/no-such-description", tasks());
    let out = host(
        &["--description", &gone, "--picture", file.to_str().unwrap()],
        "light",
    );
    let err = text(&out.stderr);
    if err.to_lowercase().contains("adapter") || err.to_lowercase().contains("graphics") {
        return;
    }
    assert!(err.contains("host-error-layout-missing"), "{err}");
    assert_eq!(out.status.code(), Some(3), "{err}");
    assert!(file.exists(), "the picture is written all the same");
}
