//! K10-K12 of ENTWURF section 5: pick, decide, precheck.
mod support;

use fs_starter::coded_stub::Coded;
use fs_starter::{Chosen, ContractReq, Next, StarterCode, Trust, Version3, decide, pick, precheck};
use support::{Demo, fid, found};

/// K10: word > person > standard.
#[test]
fn k10_pick() {
    let (word, person, standard) = (fid("w"), fid("p"), fid("simple"));
    assert_eq!(pick(Some(&word), Some(person.clone()), &standard), word);
    assert_eq!(pick(None, Some(person.clone()), &standard), person);
    assert_eq!(pick(None, None, &standard), standard);
}

fn named(code: StarterCode, frontend: &str) -> Coded<StarterCode> {
    Coded::new(code).with_text("frontend", frontend)
}

/// K11: the table of section 4.4, row by row.
#[test]
fn k11_decide_table() {
    let (std, other) = (fid("simple"), fid("other"));
    let refusal = named(StarterCode::FrontendContract, "simple").with_text("field", "extends");
    // The standard frontend: missing fails, refused opens with a hint, anything checked opens.
    assert_eq!(
        decide(&std, &std, Chosen::Missing),
        Next::Fail(named(StarterCode::FrontendMissing, "simple"))
    );
    assert_eq!(
        decide(&std, &std, Chosen::Refused(refusal.clone())),
        Next::Open(std.clone(), Some(refusal))
    );
    for answer in [None, Some(0), Some(3), Some(7)] {
        assert_eq!(
            decide(&std, &std, Chosen::Checked(answer)),
            Next::Open(std.clone(), None),
            "{answer:?}"
        );
    }
    // Another frontend: everything wrong falls back to the standard and says so.
    let fb = |c: Coded<StarterCode>| Next::Fallback(c.with_text("fallback", "simple"));
    assert_eq!(
        decide(&other, &std, Chosen::Missing),
        fb(named(StarterCode::FrontendMissing, "other"))
    );
    let refused = named(StarterCode::FrontendContract, "other").with_text("field", "fs:requires");
    assert_eq!(
        decide(&other, &std, Chosen::Refused(refused.clone())),
        fb(refused)
    );
    assert_eq!(
        decide(&other, &std, Chosen::Checked(None)),
        fb(named(StarterCode::FrontendUnstartable, "other"))
    );
    let check = named(StarterCode::FrontendContract, "other").with_text("field", "check");
    assert_eq!(decide(&other, &std, Chosen::Checked(Some(3))), fb(check));
    for answer in [0, 1, 7] {
        assert_eq!(
            decide(&other, &std, Chosen::Checked(Some(answer))),
            Next::Open(other.clone(), None),
            "{answer}"
        );
    }
}

fn field_of(r: Result<(), Coded<StarterCode>>) -> Option<String> {
    r.err().map(|e| {
        assert_eq!(e.code(), StarterCode::FrontendContract);
        support::text(&e, "field").expect("field")
    })
}

/// K12: precheck, one refusal after the other.
#[test]
fn k12_precheck() {
    let demo = Demo::default();
    let ok = found("simple", Trust::Packaged);
    assert_eq!(precheck(&ok, &demo), Ok(()));

    let mut f = ok.clone();
    f.manifest.extends = "net.freesynergy.Other".to_owned();
    assert_eq!(field_of(precheck(&f, &demo)).as_deref(), Some("extends"));

    let mut f = ok.clone();
    f.manifest.min_program = Some(Version3(0, 0, 2));
    assert_eq!(field_of(precheck(&f, &demo)).as_deref(), Some("requires"));
    f.manifest.min_program = Some(Version3(0, 0, 1));
    assert_eq!(precheck(&f, &demo), Ok(()));

    let unreadable = Demo {
        version: "x".to_owned(),
        ..Demo::default()
    };
    assert_eq!(
        field_of(precheck(&f, &unreadable)).as_deref(),
        Some("requires")
    );

    for (req, want) in [
        ("demo 2.0", Some("fs:requires")),
        ("demo 1.3", Some("fs:requires")),
        ("other 1.0", Some("fs:requires")),
        ("demo 1.2", None),
    ] {
        let mut f = ok.clone();
        f.manifest.requires = vec![ContractReq::parse(req).expect("parses")];
        assert_eq!(field_of(precheck(&f, &demo)).as_deref(), want, "{req}");
    }
}
