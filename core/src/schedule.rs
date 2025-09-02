use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Local};

/// Defines a field on the schedule for [`Schedule::Calendar`], by itself it just holds data and how this
/// data is scheduled, it is useful for [`Schedule::Calendar`] only, all fields used in the calendar are
/// zero-based (they start from zero), fields have their own ranges defined, typically:
/// - **Year** can be any value (unrestricted)
/// - **Month** must be between 0 and 11 range
/// - **Day** must be between 0 and 30 range
/// - **Hour** must be between 0 and 23
/// - **Minute** must be between 0 and 59
/// - **Second** must be between 0 and 59
/// - **Millisecond** must be between 0 and 999
///
/// All ranges are <u>inclusive on both ends</u>, the scheduler auto-validates the field schedules and if they
/// are out of bounds, it panics with the corresponding error
///
/// A field schedule has 4 variants which it can be in, these are:
/// - **Ignore** ignores the field, instead using the current time's corresponding field
/// - **Every(u32)** tells the scheduler to schedule this field on an interval basis
/// - **Exactly(u32)** tells the scheduler to schedule this field at an exact value
/// - **Custom(Arc<dyn Fn(u32) -> u32 + Send + Sync>)** triggers a custom function to run, where its argument
/// is the current time's field and returns the corresponding field to use (behaves like Exactly but as a function)
///
/// # See
/// - [`Schedule`]
/// - [`crate::scheduler::Scheduler`](scheduler)
#[derive(Clone)]
pub enum FieldSchedule {
    Ignore,
    Every(u32),
    Exactly(u32),
    Custom(Arc<dyn Fn(u32) -> u32 + Send + Sync>),
}

impl Debug for FieldSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("FieldSchedule::{}", &{
            match self {
                Self::Ignore => "Ignore".to_owned(),
                Self::Every(d) => format!("Every({d:?})"),
                Self::Exactly(res) => format!("Exactly({res:?})"),
                Self::Custom(_) => "Custom(...)".to_owned()
            }
        }))
    }
}


/// [`Schedule`] defines when a task should be executed, they have 3 forms:
/// - **Every(Duration)** executes a task on an interval basis
/// - **Cron(String)** executes a task based on the provided cron expression as a string
/// - **Calendar {...}** defines a human-friendly schedule on when the task runs, it provides fine-grain
/// control on each individual field via [`FieldSchedule`], it can be at an exact date, an interval basis... etc.
/// It is a good alternative to cron, as it provides second and millisecond accuracy plus being more human-friendly
///
/// By themselves, schedules do nothing, they are only valuable when used in a [`Scheduler`], schedules
/// can be validated via the [`Schedule::validate`] function (which the scheduler may internally use as well),
/// it is generally advised to validate the schedule before handing it over to the [`Scheduler`] and gracefully
/// handling any errors to prevent panics at runtime
///
/// # See
/// - [`FieldSchedule`]
/// - [`Scheduler`]
#[derive(Clone)]
pub enum Schedule {
    /// Execute on an interval basis, based on the supplied [`Duration`]
    Every(Duration),

    /// Defines a calendar, giving fine-grain control over each individual time field via [`FieldSchedule`],
    Calendar {
        /// The field for the year, it is the only unrestricted field as it can be any value
        year: FieldSchedule,

        /// The field of the month, it has to be in the range of 0 to 11 (inclusive both)
        month: FieldSchedule,

        /// The field of the month, it has to be in the range of 0 to 30 (inclusive both), however,
        /// be warned there are exceptions such as February which restrict the range
        day: FieldSchedule,

        /// The field of the hour, it has to be in the range of 0 to 23 (inclusive both), however,
        /// be warned there may be exceptions to this range (usually in daylight saving hour cases)
        hour: FieldSchedule,

        /// The field of the minute, it has to be in the range of 0 to 59 (inclusive both)
        minute: FieldSchedule,

        /// The field of the second, it has to be in the range of 0 to 59 (inclusive both)
        second: FieldSchedule,

        /// The field of the millisecond, it has to be in the range of 0 to 999 (inclusive both)
        millisecond: FieldSchedule
    },

    /// Executes a task based on a supplied cron expression (represented as a string)
    Cron(String)
}

impl Schedule {
    /// Validates a schedule on a specific time, it returns a Result which can be nothing
    /// or contain an error. The scheduler internally uses this and panics if it finds any errors
    pub fn validate(&self, dt: DateTime<Local>) {
        match self {
            Self::Cron(str) => {
                cron_parser::parse(str, &dt).unwrap();
            },
            Self::Every(d) => {}
            Schedule::Calendar {
                year,
                month,
                day,
                hour,
                minute,
                second,
                millisecond
            } => {
                
            }
        }
    }
}