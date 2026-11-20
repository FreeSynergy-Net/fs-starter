//! K16 of ENTWURF section 5, the starter's part: every code exactly once, kebab names, keys.
use std::collections::HashSet;

use fs_starter::StarterCode;
use fs_types::coded::{ErrorCode, message_key};

/// K16: `ALL` holds every variant once, `as_str` is unique kebab case, `message_key` is prefix and name.
#[test]
fn k16_starter_codes() {
    assert_eq!(StarterCode::ALL.len(), 14);
    let unique: HashSet<_> = StarterCode::ALL.iter().collect();
    assert_eq!(unique.len(), StarterCode::ALL.len());
    let names: HashSet<_> = StarterCode::ALL.iter().map(|c| c.as_str()).collect();
    assert_eq!(names.len(), StarterCode::ALL.len());
    for c in StarterCode::ALL {
        let n = c.as_str();
        assert!(
            !n.is_empty()
                && n.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
                && !n.starts_with('-')
                && !n.ends_with('-'),
            "{n}"
        );
        assert_eq!(message_key(*c), format!("starter-error-{n}"));
    }
}
