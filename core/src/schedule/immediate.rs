use crate::schedule::TaskSchedule;
use chrono::{DateTime, Local};
use std::sync::Arc;

/// [`TaskScheduleImmediate`] is an implementation of the [`TaskSchedule`] trait that executes tasks
/// immediately
///
/// # Construction
/// You can simply drop in the TaskScheduleImmediate, as no data is associated with it
///
/// # See also
/// - [`TaskSchedule`] - the trait implemented by this type
#[derive(Debug, Clone)]
pub struct TaskScheduleImmediate;

impl TaskSchedule for TaskScheduleImmediate {
    fn next_after(
        &self,
        time: &DateTime<Local>,
    ) -> Result<DateTime<Local>, Arc<(dyn std::error::Error + 'static)>> {
        Ok(time.clone())
    }
}
