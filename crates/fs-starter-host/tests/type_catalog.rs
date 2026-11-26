//! The type catalog and the words of a module, read from the files of the module `person` (fs-modules-person main 6fe654bc:
//! `catalog/types.bin`, and `texts/{en,de}.mf2` packed by `fs-release pack-texts` into `catalog/texts/`): the host hands the window the
//! description of the type and the words, so a head of the list is a sentence of the module and not the name of a field.
//!
//! Pattern: Characterization test (real files of a module in, the answers of the two readers out; the oracle is the module's own catalog).

use std::path::Path;

use fs_components_words::{Args, Locale, TextId};
use fs_forms_bind::{Basic, FormRef, ModuleForm};
use fs_forms_run::{FormData, FormRecords, input_of};
use fs_starter_host::sources::types;
use fs_starter_host::sources::words::PackedWords;

fn person() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/person"))
}

#[test]
fn the_catalog_of_person_describes_the_profile_with_word_ids() {
    let registry = types::read(person()).unwrap().expect("a catalog");
    let profile = registry.get("Profile").expect("the type Profile");
    let emails = profile
        .fields
        .iter()
        .find(|f| f.name == "emails")
        .expect("emails");
    assert_eq!(emails.label, "person.emails");
}

#[test]
fn the_words_of_person_answer_in_english_and_german() {
    let words = PackedWords::of(person(), "person").expect("the module brings words");
    let say = |locale: &str| {
        words
            .in_locale(&Locale::parse(locale).unwrap())
            .say(&TextId::new("person.emails"), &Args::new())
            .text()
            .to_owned()
    };
    assert_eq!(say("en"), "Email addresses");
    assert_eq!(say("de"), "E-Mail-Adressen");
}

#[test]
fn a_module_without_catalog_texts_brings_no_words() {
    let dir = tempfile::tempdir().unwrap();
    assert!(PackedWords::of(dir.path(), "none").is_none());
}

fn form(data_type: &str) -> FormData {
    let mut data = FormData::read_only(
        ModuleForm {
            data_type: data_type.into(),
            form: FormRef {
                name: "rows".into(),
                major: 1,
                minor: 0,
            },
            map: vec![("columns".into(), vec!["emails".into()])],
            fields: vec![("emails".into(), Basic::Email)],
        },
        Vec::new(),
    );
    // the pages read set it from the wire (`read_one`)
    data.data_type = data_type.into();
    data
}

#[test]
fn each_form_gets_the_description_of_its_own_type_and_the_heads_the_word_id() {
    let registry = types::read(person()).unwrap().unwrap();
    let mut records = FormRecords {
        forms: vec![form("Profile"), form("Lane")],
        ..FormRecords::default()
    };
    types::attach(&mut records, &registry);
    let label = |i: usize| {
        input_of(&records.forms[i]).channels[0].fields[0]
            .label
            .clone()
    };
    assert_eq!(label(0), "person.emails");
    assert!(
        records.forms[1].type_description.is_none(),
        "a type the catalog lacks stays plain"
    );
    assert_eq!(label(1), "");
}
