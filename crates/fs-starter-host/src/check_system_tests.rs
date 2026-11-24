//! Unit tests of `--check-system` (the words and the one-line rule).
//!
//! Pattern: none (plain tests of two private functions).

use super::*;

fn w(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn the_words_are_read_and_a_wrong_word_is_refused() {
    let got = words(&w(&[
        "--check-system",
        "--modules",
        "/m",
        "--program",
        "a",
        "--program",
        "b",
    ]))
    .unwrap();
    assert_eq!(got.modules, Some(PathBuf::from("/m")));
    assert_eq!(got.programs, ["a", "b"]);
    assert!(words(&w(&["--check-system", "--bogus"])).is_err());
    assert!(words(&w(&["--check-system", "--modules"])).is_err());
    assert!(words(&w(&["--check-system", "--modules", "a", "--modules", "b"])).is_err());
}

#[test]
fn a_free_text_never_makes_a_second_line() {
    assert_eq!(val("a\nred x=1"), "\"a\\nred x=1\"");
}
