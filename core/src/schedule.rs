pub mod calendar;
pub mod cron;
pub mod immediate;
pub mod interval;

pub use crate::schedule::calendar::TaskCalendarField;
pub use crate::schedule::calendar::TaskScheduleCalendar;
pub use crate::schedule::cron::TaskScheduleCron;
pub use crate::schedule::immediate::TaskScheduleImmediate;
pub use crate::schedule::interval::TaskScheduleInterval;

use chrono::{DateTime, Local};
use std::error::Error;
use std::sync::Arc;

/// The [`TaskSchedule`] trait defines when a task should be executed, by default they can be in 3 forms:
/// - [`TaskScheduleInterval`] executes a task on an interval basis
/// - [`TaskScheduleCalendar`] executes a task based on the provided cron expression as a string
/// - [`TaskScheduleCron`] defines a human-friendly schedule on when the task runs, it provides fine-grain
///   control on each individual field via [`TaskCalendarField`], it can be at an exact date, an interval basis... etc.
///   It is a good alternative to cron, as it provides second and millisecond accuracy plus being more human-friendly
///
/// The schedule by itself is only useful when used together with a [`Scheduler`]
///
/// # See
/// - [`Scheduler`]
/// - [`TaskScheduleInterval`]
/// - [`TaskScheduleCalendar`]
/// - [`TaskScheduleCron`]
pub trait TaskSchedule: Send + Sync {
    /// Calculates the future time to execute, this may return an error in the process if unable due
    /// to any reason, read more on the trait implementation's documentation to learn more
    fn next_after(&self, time: &DateTime<Local>) -> Result<DateTime<Local>, Arc<dyn Error>>;
}

impl<TS: TaskSchedule + ?Sized> TaskSchedule for Arc<TS> {
    fn next_after(&self, time: &DateTime<Local>) -> Result<DateTime<Local>, Arc<dyn Error>> {
        self.as_ref().next_after(time)
    }
}
