//! The borders of an answer (plan section 4, border 5) at the place where the records of a page are turned into rows by channel:
//! the count, the bytes of one text, the values of one channel, the bytes of everything, and the masking of control characters.
//!
//! Pattern: Table of cases (each border one test, a mutant that removes it must turn one red).

use fs_forms_bind::{Basic, FormRef, ModuleForm};
use fs_interfaces::formsource_capnp::record_page;

use super::super::answer::Limits;
use super::rows;

fn form() -> ModuleForm {
    ModuleForm {
        data_type: String::new(),
        form: FormRef {
            name: "tree".into(),
            major: 1,
            minor: 0,
        },
        map: vec![("label".into(), vec!["name".into()])],
        fields: vec![("name".into(), Basic::Text)],
    }
}

/// A page of `n` records, each with the values `(path, text)`.
fn page(n: u32, values: &[(&str, &str)]) -> capnp::message::Builder<capnp::message::HeapAllocator> {
    let mut msg = capnp::message::Builder::new_default();
    let root = msg.init_root::<record_page::Builder>();
    let mut list = root.init_records(n);
    for i in 0..n {
        let mut r = list.reborrow().get(i);
        r.set_id(format!("r{i}").as_str());
        let mut vs = r.init_values(u32::try_from(values.len()).unwrap());
        for (at, (path, text)) in values.iter().enumerate() {
            let mut v = vs.reborrow().get(u32::try_from(at).unwrap());
            v.set_path(path);
            v.set_text(text);
        }
    }
    msg
}

fn read(
    msg: &capnp::message::Builder<capnp::message::HeapAllocator>,
    limits: &Limits,
) -> Result<Vec<fs_forms_run::FormRow>, ()> {
    let reader = msg.get_root_as_reader::<record_page::Reader>().unwrap();
    rows(reader.get_records().unwrap(), &form(), limits, &mut 0)
}

#[test]
fn control_characters_are_masked_before_a_row_is_built() {
    let msg = page(1, &[("name", "a\u{1b}[31m\nb")]);
    let got = read(&msg, &Limits::default()).unwrap();
    assert_eq!(got[0].get("label"), Some("a\u{FFFD}[31m\u{FFFD}b"));
}

#[test]
fn more_records_than_the_limit_refuse_the_whole_answer() {
    let limits = Limits {
        rows: 3,
        ..Limits::default()
    };
    assert_eq!(read(&page(3, &[("name", "x")]), &limits).unwrap().len(), 3);
    assert!(read(&page(4, &[("name", "x")]), &limits).is_err());
}

#[test]
fn a_long_text_is_cut_and_the_values_of_a_channel_are_limited() {
    let limits = Limits {
        text: 4,
        values: 2,
        ..Limits::default()
    };
    let msg = page(1, &[("name", "abcdefgh"), ("name", "b"), ("name", "c")]);
    let got = read(&msg, &limits).unwrap();
    assert_eq!(got[0].all("label"), ["abcd", "b"]);
}

#[test]
fn the_bytes_of_all_records_together_are_limited() {
    let limits = Limits {
        bytes: 40,
        ..Limits::default()
    };
    assert!(read(&page(2, &[("name", "0123456789")]), &limits).is_ok());
    assert!(read(&page(5, &[("name", "0123456789")]), &limits).is_err());
}

#[test]
fn only_the_fields_the_module_mapped_are_taken() {
    let got = read(
        &page(1, &[("secret", "x"), ("name", "y")]),
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(got[0].values.len(), 1);
    assert_eq!(got[0].get("label"), Some("y"));
}

#[test]
fn the_may_of_a_record_is_read_masked_and_limited() {
    let mut msg = capnp::message::Builder::new_default();
    let root = msg.init_root::<record_page::Builder>();
    let list = root.init_records(1);
    let mut r = list.get(0);
    r.set_id("r0");
    let mut may = r.init_may(3);
    may.set(0, "card.move");
    may.set(1, "card\u{1b}.drop");
    may.set(2, "third");
    let limits = Limits {
        values: 2,
        ..Limits::default()
    };
    let got = read(&msg, &limits).unwrap();
    assert_eq!(got[0].may, ["card.move", "card\u{FFFD}.drop"]);
}
