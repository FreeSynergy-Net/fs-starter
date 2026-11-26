//! The "now" of a view of sample data (F4b): the host gives the records the clock of the sample set (`FormRecords::clock`), and
//! `--sample-now` (read by the window host) overrides it. No `--sample` special branch: the window reads the time only through the
//! port `Clock` of the opened records, as with a real module (the system clock).
//!
//! Pattern: none - example tests through the port `open` of `fs-forms-run`, as the window calls it.
#![cfg(unix)]

use std::path::Path;

use fs_forms_exchange::{Format, read_layout};
use fs_forms_layout::{Layout, Limits, OpenCatalog};
use fs_forms_run::{
    CatalogBlockForms, Clock, Command, Described, Moment, SystemClock, open, parse, session_locale,
    view_of, words,
};
use fs_forms_testdata::SampleSpec;
use fs_starter_host::registry::SystemPackaged;
use fs_starter_host::sources::{Limits as Borders, Modules, StarterSources};

const CALENDAR: &str = include_str!("data/calendar-sample.toml");

fn layout() -> Layout {
    read_layout(
        &mut CALENDAR.as_bytes(),
        Path::new("board.toml"),
        Format::Toml,
    )
    .expect("reads")
    .check(&OpenCatalog, &Limits::declared())
    .expect("checks")
}

fn sources() -> StarterSources {
    let modules = Modules::load(
        Path::new("/nonexistent/bundled"),
        Path::new("/nonexistent/system"),
        &SystemPackaged::new(),
    );
    StarterSources::new(modules, Borders::default())
}

fn moment(text: &str) -> Moment {
    text.parse().expect("rfc3339")
}

/// The "now" the opened window takes for these words of the command line.
fn now_of(extra: &[&str]) -> Moment {
    let layout = layout();
    let view = view_of(&layout, "en");
    let mut all = vec!["--description".to_owned(), "x.toml".to_owned()];
    all.extend(extra.iter().map(|s| (*s).to_owned()));
    let Ok(Command::Window(c)) = parse(&all) else {
        panic!("not a window: {extra:?}");
    };
    let texts = words(&session_locale());
    open(
        &c,
        &view,
        &Described::of(Some(&layout)),
        &texts,
        Path::new("x"),
        &sources(),
    )
    .clock
    .now()
}

/// What the sample set itself says it is: the moment the data were made.
fn set_now() -> Moment {
    let layout = layout();
    let described = Described::of(Some(&layout));
    let specs = SampleSpec::from_description(&layout, &CatalogBlockForms, described.sample())
        .expect("spec");
    let spec = specs.first().expect("one source");
    spec.build()
        .expect("builds")
        .now_utc(&spec.spec.anchor.zone)
        .expect("zone")
        .expect("the set has a now")
}

#[test]
fn sample_data_without_the_switch_show_the_now_of_the_sample_set() {
    let want = set_now();
    assert_eq!(now_of(&["--sample-data"]), want);
    assert!(
        want < moment("2026-11-25T00:00:00Z"),
        "the set is older than the picture day: {want}"
    );
}

#[test]
fn the_switch_gives_the_view_a_now_later_than_the_data() {
    let want = moment("2026-11-25T09:00:00Z");
    assert_eq!(
        now_of(&["--sample-data", "--sample-now", "2026-11-25T09:00:00Z"]),
        want
    );
    assert_ne!(now_of(&["--sample-data"]), want);
}

#[test]
fn an_invalid_value_is_refused_and_nothing_starts() {
    for bad in [
        &["--sample-data", "--sample-now", "tomorrow"][..],
        &["--sample-data", "--sample-now"][..],
        &["--sample-now", "2026-11-25T09:00:00Z"][..],
    ] {
        let mut all = vec!["--description".to_owned(), "x.toml".to_owned()];
        all.extend(bad.iter().map(|s| (*s).to_owned()));
        assert!(parse(&all).is_err(), "{bad:?} must be refused");
    }
}

#[test]
fn without_sample_data_the_view_reads_the_system_clock() {
    let got = now_of(&[]);
    assert!(got > moment("2026-01-01T00:00:00Z"), "{got}");
    assert!(got <= SystemClock.now());
}
