//! The pins of the lifted libraries, read from `Cargo.lock` (K4b, mutant P4: `fs-conformance` back on the `rev` of fs-types main 339aa24
//! has no `fs-types` of its own and so keeps "one source" green; only the exact tag shows it).

/// The doubles that are KNOWN and wait for a follow-up commit of their owner. A line here is allowed only with the reason and the owner
/// that has to move; the test below fails when it is no longer needed.
/// pin-13a: `fs-forms-client` and `fs-forms-run` (fs-forms 04e22e2a, C2) still pin fs-rpc dba7125f (wave 12); the host and the services
/// (planning, person) are on 7a656f45 (wave 13: the same code, only its own fs-components pin moved). The owner moves: fs-forms.
const KNOWN_OLD: [(&str, &str); 2] = [
    ("fs-rpc", "rev=dba7125f4c8baa51d55786bbbff150b9052c35a1"),
    (
        "fs-trusted-path",
        "rev=dba7125f4c8baa51d55786bbbff150b9052c35a1",
    ),
];

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
            "REV:92dbb53e9c9ea2c48e85a27712f0456f2a7b8161",
        ),
        ("fs-objects", "REV:e38093f1ec61b7c9faaa6f6046f65addb11dcf46"),
        (
            "fs-settings-rpc",
            "REV:7a656f456f3f05805f1fa2c128b21837ebd21eb9",
        ),
        (
            "fs-settings",
            "REV:08d501d9d81a111ece4b95115eb4540223c83fac",
        ),
        (
            "fs-components-commands",
            "REV:08d501d9d81a111ece4b95115eb4540223c83fac",
        ),
        ("fs-rpc", "REV:7a656f456f3f05805f1fa2c128b21837ebd21eb9"),
        (
            "fs-components-kanban",
            "REV:08d501d9d81a111ece4b95115eb4540223c83fac",
        ),
        (
            "fs-trusted-path",
            "REV:7a656f456f3f05805f1fa2c128b21837ebd21eb9",
        ),
        (
            "fs-forms-run",
            "REV:04e22e2a5362fadb3b7f53bfe8a9ad54f859ef1d",
        ),
        (
            "fs-forms-bind",
            "REV:04e22e2a5362fadb3b7f53bfe8a9ad54f859ef1d",
        ),
        (
            "fs-forms-testdata",
            "REV:04e22e2a5362fadb3b7f53bfe8a9ad54f859ef1d",
        ),
        (
            "fs-forms-layout",
            "REV:04e22e2a5362fadb3b7f53bfe8a9ad54f859ef1d",
        ),
        (
            "fs-forms-exchange",
            "REV:04e22e2a5362fadb3b7f53bfe8a9ad54f859ef1d",
        ),
        (
            "fs-components",
            "REV:08d501d9d81a111ece4b95115eb4540223c83fac",
        ),
        ("fs-module", "REV:92dbb53e9c9ea2c48e85a27712f0456f2a7b8161"),
        (
            "fs-modules-planning",
            "REV:8b1e616d075958757a04d6b824d8580680a10b78",
        ),
        (
            "fs-tasks-core",
            "REV:8b1e616d075958757a04d6b824d8580680a10b78",
        ),
        ("fs-person", "REV:bfebf590689d21d64851661b39e52fee286a617d"),
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
