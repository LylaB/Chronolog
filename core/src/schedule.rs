pub mod interval;
pub mod calendar;
pub mod cron;

pub use crate::schedule::interval::ScheduleInterval;
pub use crate::schedule::calendar::ScheduleCalendar;
pub use crate::schedule::calendar::CalendarFieldSchedule;
pub use crate::schedule::cron::ScheduleCron;

use std::error::Error;
use std::sync::Arc;
use chrono::{DateTime, Local};

/// The [`Schedule`] trait defines when a task should be executed, by default they can be in 3 forms:
/// - [`ScheduleInterval`] executes a task on an interval basis
/// - [`ScheduleCalendar`] executes a task based on the provided cron expression as a string
/// - [`ScheduleCron`] defines a human-friendly schedule on when the task runs, it provides fine-grain
/// control on each individual field via [`FieldSchedule`], it can be at an exact date, an interval basis... etc.
/// It is a good alternative to cron, as it provides second and millisecond accuracy plus being more human-friendly
///
/// The schedule by itself is only useful when used together with a [`Scheduler`]
///
/// # See
/// - [`Scheduler`]
/// - [`ScheduleInterval`]
/// - [`ScheduleCalendar`]
/// - [`ScheduleCron`]
pub trait Schedule: Send + Sync {
    /// Calculates the future time to execute, this may return an error in the process if unable due
    /// to any reason, read more on the trait implementation's documentation to learn more
    fn next_after(&self, time: DateTime<Local>) -> Result<DateTime<Local>, Arc<dyn Error>>;
}