use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Local, TimeDelta};
use crate::schedule::Schedule;

/// [`ScheduleInterval`] is a straightforward implementation of the [`Schedule`] trait
/// that executes tasks at a fixed interval.
///
/// The interval is defined using either a [`TimeDelta`] or a [`Duration`], making it
/// flexible for different time representations. This makes it well-suited for recurring
/// jobs such as periodic cleanup tasks, heartbeat signals, polling operations... etc.
///
/// # Construction
///
/// - Use [`ScheduleInterval::new`] to create an interval schedule from a [`TimeDelta`].
/// - Use [`ScheduleInterval::duration`] when constructing from a [`Duration`].
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
/// use chronolog::schedule::ScheduleInterval;
///
/// // Run every 5 seconds
/// let schedule = ScheduleInterval::duration(Duration::from_secs(5));
/// ```
///
/// # See also
/// - [`Schedule`] — the trait implemented by this type
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Copy, Default)]
pub struct ScheduleInterval(pub(crate) TimeDelta);

impl ScheduleInterval {
    pub fn new(interval: TimeDelta) -> Self {
        Self(interval)
    }

    pub fn duration(interval: Duration) -> Self {
        Self(TimeDelta::from_std(interval).unwrap())
    }
}

impl Schedule for ScheduleInterval {
    fn next_after(&self, time: &DateTime<Local>) -> Result<DateTime<Local>, Arc<(dyn std::error::Error + 'static)>> {
        Ok(time.add(self.0))
    }
}