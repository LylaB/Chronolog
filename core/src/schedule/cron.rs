use std::sync::Arc;
use chrono::{DateTime, Local};
use crate::schedule::Schedule;

/// [`ScheduleCron`] is an implementation of the [`Schedule`] trait that executes tasks
/// according to a cron expression.
///
/// Cron expressions provide a powerful way to define recurring schedules with fine-grained
/// control (e.g., "every minute", "at 2:30 AM every day", "every Monday at 9 AM").
/// The expression is supplied as a string and parsed when running [`ScheduleCron::next_after`].
/// The only drawback compared to something like [`ScheduleCalendar`] is the inability to
/// have second and millisecond precision.
///
/// # Construction
///
/// Use [`ScheduleCron::new`] to create a schedule from a cron expression string.
///
/// # Examples
///
/// ```rust
/// // Run at 12:00 (noon) every day
/// use chronolog::schedule::ScheduleCron;
///
/// let schedule = ScheduleCron::new("0 12 * * *".to_owned());
///
/// // Run every 5 minutes
/// let schedule = ScheduleCron::new("*/5 * * * *".to_owned());
/// ```
///
/// # See also
/// - [`Schedule`] — the trait implemented by this type
/// - [`ScheduleCalendar`]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduleCron(pub(crate) String);

impl ScheduleCron {
    pub fn new(cron: String) -> Self {
        Self(cron)
    }
}

impl Schedule for ScheduleCron {
    fn next_after(&self, time: DateTime<Local>) -> Result<DateTime<Local>, Arc<(dyn std::error::Error + 'static)>> {
        Ok(cron_parser::parse(&*self.0, &time).unwrap())
    }
}