//! The word `--program NAME` of the host: the data package of an installed program, found only in the system place.
//!
//! Pattern: Adapter (words in, words out; the place check is `SystemPlaces`).

/// `--program NAME`: the data package of an installed program, found in the system folder of the packages
/// (`/usr/share/freesynergy/programs/<name>`) ONLY if the place passes the system check (`SystemPlaces::program_dir`: root's, no
/// symlink, not writable by others). The pair becomes `--description <folder>`; every other word stays. A name that is not a plain
/// folder name, a place that does not pass, or a second description is refused, never opened from somewhere else.
pub(crate) fn with_program(
    args: &[String],
    places: &fs_starter::SystemPlaces,
) -> Result<Vec<String>, String> {
    let Some(at) = args.iter().position(|a| a == "--program") else {
        return Ok(args.to_vec());
    };
    let name = args
        .get(at + 1)
        .ok_or_else(|| "program-package-name-missing".to_owned())?;
    if args.iter().any(|a| a == "--description") {
        return Err("program-package-and-description".to_owned());
    }
    let dir = places
        .program_dir(name)
        .map_err(|why| format!("program-package-refused: {name} ({why})"))?;
    let mut out = args.to_vec();
    out.splice(
        at..=at + 1,
        [
            "--description".to_owned(),
            dir.to_string_lossy().into_owned(),
        ],
    );
    Ok(out)
}

#[cfg(all(test, unix))]
mod program_tests {
    use super::*;
    use fs_starter::SystemPlaces;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    fn words(w: &[&str]) -> Vec<String> {
        w.iter().map(|s| (*s).to_owned()).collect()
    }

    fn places() -> (tempfile::TempDir, SystemPlaces) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("programs/groups")).unwrap();
        let owner = std::fs::metadata(dir.path()).unwrap().uid();
        let p = SystemPlaces::for_test(vec![dir.path().to_path_buf()], owner);
        (dir, p)
    }

    #[test]
    fn a_program_name_becomes_the_description_of_its_package_folder() {
        let (dir, p) = places();
        let got = with_program(&words(&["--program", "groups", "--sample-data"]), &p).unwrap();
        let folder = dir.path().join("programs/groups");
        assert_eq!(
            got,
            ["--description", folder.to_str().unwrap(), "--sample-data"]
        );
        assert_eq!(
            with_program(&words(&["--sample-data"]), &p).unwrap(),
            ["--sample-data"]
        );
    }

    #[test]
    fn a_name_a_place_or_a_second_description_that_does_not_fit_is_refused() {
        let (dir, p) = places();
        for bad in ["../x", "a/b", "", "Groups", "nope"] {
            assert!(
                with_program(&words(&["--program", bad]), &p).is_err(),
                "{bad}"
            );
        }
        assert!(with_program(&words(&["--program"]), &p).is_err());
        assert!(with_program(&words(&["--program", "groups", "--description", "x"]), &p).is_err());
        // writable by others: not a system place
        let folder = dir.path().join("programs/groups");
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o777)).unwrap();
        let e = with_program(&words(&["--program", "groups"]), &p).unwrap_err();
        assert!(e.contains("writable"), "{e}");
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755)).unwrap();
        // a link in its place
        std::fs::remove_dir(&folder).unwrap();
        std::os::unix::fs::symlink("/tmp", &folder).unwrap();
        assert!(
            with_program(&words(&["--program", "groups"]), &p)
                .unwrap_err()
                .contains("symlink")
        );
    }
}
