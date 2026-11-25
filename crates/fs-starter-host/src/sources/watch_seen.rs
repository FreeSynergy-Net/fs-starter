//! What a watch has seen of the feed: the watermark and the latest activities, so one change is counted ONCE whoever tells it first (the new
//! watch or the catch-up through `changes`).
//!
//! Pattern: Memento (the state of the feed as far as it was seen) + Value Object.

use std::collections::VecDeque;

/// How many activities are remembered for the duplicate check (anything newer than the watermark of an end can only be seen again soon).
const RECENT: usize = 1024;

/// What was seen of the feed: the watermark (activities are `UUIDv7`: the larger is the later) and the latest ones for the duplicate check.
#[derive(Default)]
pub(super) struct Seen {
    pub(super) high: Option<u128>,
    recent: VecDeque<u128>,
}

impl Seen {
    /// `true` the first time `activity` is seen.
    pub(super) fn first(&mut self, activity: u128) -> bool {
        if self.recent.contains(&activity) {
            return false;
        }
        if self.recent.len() == RECENT {
            self.recent.pop_front();
        }
        self.recent.push_back(activity);
        self.high = Some(self.high.map_or(activity, |h| h.max(activity)));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_activity_is_first_once_and_the_watermark_is_the_largest() {
        let mut s = Seen::default();
        assert_eq!(s.high, None);
        assert!(s.first(5));
        assert!(s.first(9));
        assert!(!s.first(5));
        assert!(s.first(7));
        assert_eq!(s.high, Some(9));
    }

    #[test]
    fn only_the_latest_are_remembered() {
        let mut s = Seen::default();
        for i in 1..=(RECENT as u128 + 1) {
            assert!(s.first(i));
        }
        assert!(s.first(1), "the oldest was forgotten");
        assert_eq!(s.recent.len(), RECENT);
    }
}
