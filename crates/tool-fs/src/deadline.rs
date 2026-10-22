//! [`Deadline`]: the wall-clock bound of one call (`bounds.timeout_ms`), checked in every loop
//! that reads, and remembered once it has passed so the reply can say so.
//!
//! Public so that every toolset that reads the tree stops the same way: the `search` toolset hands
//! one to [`walk_within`](crate::walk::walk_within) and checks it per line, as `fs.glob` and
//! `fs.read` do.

use std::cell::Cell;
use std::time::{Duration, Instant};

/// The `timeout_ms` a call gets when it names none, as the `fs` and `search` contracts declare it.
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
/// The largest `timeout_ms` a call may name; a larger one is refused, as the `fs` and `search`
/// contracts declare it.
pub const MAX_TIMEOUT_MS: u64 = 60_000;

/// When one call has to stop, and whether a check has found that it had to.
///
/// Every [`Deadline::passed`] is one check. A deadline that has passed stays passed, so the
/// operation that stopped on it reads [`Deadline::reached`] afterwards and answers
/// `deadline_reached` from that, not from a second look at the clock.
///
/// A deadline of 0 milliseconds has passed at its first check, so a call that names
/// `timeout_ms: 0` stops before it reads anything, and it does so on every run.
#[derive(Debug)]
pub struct Deadline {
    stop: Stop,
    reached: Cell<bool>,
}

#[derive(Debug)]
enum Stop {
    Never,
    At(Instant),
    AfterChecks(Cell<u64>),
}

impl Deadline {
    /// A deadline `timeout` from now. One too far in the future to represent never passes.
    pub fn after(timeout: Duration) -> Self {
        Self::of(
            Instant::now()
                .checked_add(timeout)
                .map_or(Stop::Never, Stop::At),
        )
    }

    /// A deadline `timeout_ms` milliseconds from now.
    pub fn after_ms(timeout_ms: u64) -> Self {
        Self::after(Duration::from_millis(timeout_ms))
    }

    /// A deadline that never passes.
    pub fn never() -> Self {
        Self::of(Stop::Never)
    }

    /// A deadline that passes at check `checks + 1`, whatever the clock says: `after_checks(0)`
    /// stops at the first check, as `timeout_ms: 0` does. For a test that has to stop a loop at a
    /// known place, which a clock cannot do on every machine.
    pub fn after_checks(checks: u64) -> Self {
        Self::of(Stop::AfterChecks(Cell::new(checks)))
    }

    fn of(stop: Stop) -> Self {
        Self {
            stop,
            reached: Cell::new(false),
        }
    }

    /// One check: whether the deadline has passed. Once it answers `true` it always does.
    pub fn passed(&self) -> bool {
        if self.reached.get() {
            return true;
        }
        let passed = match &self.stop {
            Stop::Never => false,
            Stop::At(at) => Instant::now() >= *at,
            Stop::AfterChecks(left) => match left.get() {
                0 => true,
                more => {
                    left.set(more - 1);
                    false
                }
            },
        };
        self.reached.set(passed);
        passed
    }

    /// Whether a check has found the deadline passed. Not a check itself.
    pub fn reached(&self) -> bool {
        self.reached.get()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Deadline;

    #[test]
    fn a_deadline_of_zero_has_passed_at_its_first_check_and_stays_passed() {
        let deadline = Deadline::after(Duration::ZERO);
        assert!(!deadline.reached());
        assert!(deadline.passed());
        assert!(deadline.reached());
        assert!(deadline.passed());
    }

    #[test]
    fn a_deadline_after_checks_passes_at_the_check_after_them() {
        let deadline = Deadline::after_checks(2);
        assert!(!deadline.passed());
        assert!(!deadline.passed());
        assert!(!deadline.reached());
        assert!(deadline.passed());
        assert!(deadline.reached());
    }

    #[test]
    fn a_deadline_far_away_and_one_that_never_comes_do_not_pass() {
        assert!(!Deadline::after(Duration::from_secs(3600)).passed());
        assert!(!Deadline::after(Duration::MAX).passed());
        assert!(!Deadline::never().passed());
    }
}
