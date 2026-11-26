//! The pins of the lifted libraries, read from `Cargo.lock` (K4b, mutant P4: `fs-conformance` back on the `rev` of fs-types main 339aa24
//! has no `fs-types` of its own and so keeps "one source" green; only the exact tag shows it).

/// The doubles that are KNOWN and wait for a follow-up commit of their owner: none (E3 final, 24.11.: every library has one source).
/// A line here is allowed only with the reason and the owner that has to move; the test below fails when it is no longer needed.
const KNOWN_OLD: [(&str, &str); 0] = [];

/// The `source` line of the package `name` in the workspace lock (there must be exactly one, apart from the `KNOWN_OLD` doubles).
fn source_of(name: &str) -> String {
    let lock = include_str!("../../../Cargo.lock");
    let mut found = Vec::new();
    for block in lock.split("[[package]]") {
        if block
            .lines()
            .any(|l| l.trim() == format!("name = \"{name}\""))
        {
            let source = block
                .lines()
                .find_map(|l| l.strip_prefix("source = "))
                .unwrap_or("")
                .trim_matches('"')
                .to_owned();
            found.push(source);
        }
    }
    let old: Vec<&str> = KNOWN_OLD
        .iter()
        .filter(|(n, _)| *n == name)
        .map(|(_, s)| *s)
        .collect();
    found.retain(|s| !old.iter().any(|o| s.contains(o)));
    assert_eq!(
        found.len(),
        1,
        "{name}: one source in the lock, found {found:?}"
    );
    found.remove(0)
}

/// Each lifted library is its tag, or (value "REV:<sha>") the exact commit of a main head not yet tagged (fs-forms, fs-components).
#[test]
fn the_lifted_libraries_are_pinned_by_their_tags_or_commits() {
    for (name, tag) in [
        ("fs-types", "REV:722d3ce0156711d40cb278f2e835e865d9448290"),
        (
            "fs-conformance",
            "REV:722d3ce0156711d40cb278f2e835e865d9448290",
        ),
        (
            "fs-interfaces",
            "REV:2354f440985bf4c82d0a725928f8576032a595c6",
        ),
        ("fs-objects", "REV:e38093f1ec61b7c9faaa6f6046f65addb11dcf46"),
        (
            "fs-settings-rpc",
            "REV:6e80162ace04e3b3eed849b1004c175b73460ce2",
        ),
        (
            "fs-settings",
            "REV:bd170fa086ec231e1eb724596849b66fb6a75320",
        ),
        (
            "fs-components-commands",
            "REV:bd170fa086ec231e1eb724596849b66fb6a75320",
        ),
        ("fs-rpc", "REV:6e80162ace04e3b3eed849b1004c175b73460ce2"),
        (
            "fs-components-kanban",
            "REV:bd170fa086ec231e1eb724596849b66fb6a75320",
        ),
        (
            "fs-trusted-path",
            "REV:6e80162ace04e3b3eed849b1004c175b73460ce2",
        ),
        (
            "fs-forms-run",
            "REV:4a549f114e28fa1cfdd2b51c792ad622994165c6",
        ),
        (
            "fs-forms-bind",
            "REV:4a549f114e28fa1cfdd2b51c792ad622994165c6",
        ),
        (
            "fs-forms-testdata",
            "REV:4a549f114e28fa1cfdd2b51c792ad622994165c6",
        ),
        (
            "fs-forms-layout",
            "REV:4a549f114e28fa1cfdd2b51c792ad622994165c6",
        ),
        (
            "fs-forms-exchange",
            "REV:4a549f114e28fa1cfdd2b51c792ad622994165c6",
        ),
        (
            "fs-components",
            "REV:bd170fa086ec231e1eb724596849b66fb6a75320",
        ),
        ("fs-module", "REV:2354f440985bf4c82d0a725928f8576032a595c6"),
        (
            "fs-modules-planning",
            "REV:6b52a824d8d0505651b255c43f8eac846f38905a",
        ),
        (
            "fs-tasks-core",
            "REV:6b52a824d8d0505651b255c43f8eac846f38905a",
        ),
        ("fs-person", "REV:f9325bba150a7c0c5eea3f25c80b83bb045cf9f4"),
    ] {
        let source = source_of(name);
        let want = match tag.strip_prefix("REV:") {
            Some(sha) => format!("?rev={sha}#{sha}"),
            None => format!("?tag={tag}#"),
        };
        assert!(source.contains(&want), "{name}: {source} is not {tag}");
    }
}

/// `fs-conformance` lives in the repo fs-types: the same commit as `fs-types` (one source, and the right one).
#[test]
fn the_conformance_is_the_commit_of_fs_types() {
    assert_eq!(source_of("fs-conformance"), source_of("fs-types"));
}

/// `fs-rpc` and `fs-trusted-path` are two crates of one repository: the same repository URL and the same commit (one source each).
#[test]
fn the_rpc_and_the_trusted_path_come_from_one_repository_at_one_commit() {
    assert_eq!(source_of("fs-rpc"), source_of("fs-trusted-path"));
    assert!(source_of("fs-rpc").contains("github.com/FreeSynergy-Net/fs-rpc?rev="));
}

/// A `KNOWN_OLD` line that the lock no longer has is removed (the double is gone: the one-source rule is strict again).
#[test]
fn every_known_old_double_is_still_in_the_lock() {
    let lock = include_str!("../../../Cargo.lock");
    for (name, source) in KNOWN_OLD {
        let has = lock.split("[[package]]").any(|b| {
            b.lines().any(|l| l.trim() == format!("name = \"{name}\"")) && b.contains(source)
        });
        assert!(
            has,
            "{name} {source} is gone from the lock: delete the line of KNOWN_OLD"
        );
    }
}
