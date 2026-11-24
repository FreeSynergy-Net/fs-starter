//! The level gate of the host (`FormSource`, `fs_interfaces::forms`): levels are CUMULATIVE. A service that says a level `>=
//! FORM_SOURCE_MIN_LEVEL` is taken, the host works with `min(level, FORM_SOURCE_LEVEL)` (a newer service still works, only what the host
//! knows is used); a service below the minimum is refused with the reason. Level 2 is what adds `invokeIf`, `revert`, `introduce` and `watch`.
//!
//! Pattern: Guard (a pure check before anything is used) + Strategy by level (the caller picks the way of level 1 or level 2).

use fs_interfaces::forms::{FORM_SOURCE_LEVEL, FORM_SOURCE_MIN_LEVEL};

/// The level a service said and the one the host works with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Levels {
    pub service: u32,
    pub used: u32,
}

/// The first level that has history (`invokeIf`, `revert`), `introduce` and `watch`.
pub(super) const HISTORY_FROM: u32 = 2;

/// The level the host works with for a service that says `service`; `None` = below the minimum (refused).
#[must_use]
pub(super) fn negotiate(service: u32) -> Option<u32> {
    (service >= FORM_SOURCE_MIN_LEVEL).then_some(service.min(FORM_SOURCE_LEVEL))
}

/// The ONE line that shows the negotiated level: `fs-starter: level module=M service=S used=U`.
#[must_use]
pub(super) fn line(module: &str, service: u32, used: u32) -> String {
    format!("fs-starter: level module={module} service={service} used={used}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_is_cumulative_and_refuses_below_the_minimum() {
        // counter-proof: the old exact test `level == FORM_SOURCE_LEVEL` would refuse 1 and 99; the gate takes both
        assert_eq!(
            negotiate(FORM_SOURCE_MIN_LEVEL),
            Some(FORM_SOURCE_MIN_LEVEL)
        );
        assert_eq!(negotiate(FORM_SOURCE_LEVEL), Some(FORM_SOURCE_LEVEL));
        assert_eq!(negotiate(99), Some(FORM_SOURCE_LEVEL), "min(level, LEVEL)");
        assert_eq!(negotiate(FORM_SOURCE_MIN_LEVEL - 1), None);
    }

    #[test]
    fn the_status_line_is_one_line_with_both_numbers() {
        let l = line("planning", 99, 2);
        assert_eq!(l, "fs-starter: level module=planning service=99 used=2");
        assert!(!l.contains('\n'));
    }
}
