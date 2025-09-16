pub mod virtual_clock;
pub mod system_clock;

pub use virtual_clock::VirtualClock;
pub use system_clock::SystemClock;

use std::time::{SystemTime, Duration};
use async_trait::async_trait;
use chrono::{DateTime, OutOfRangeError, TimeDelta, TimeZone};


/// [`SchedulerClock`] is a trait for implementing a custom scheduler clock, typical operations
/// include getting the current time, idle for a specific duration (or til a specific date is reached).
/// As it goes for implementations, by default, there are 2 of them:
///
/// - [`VirtualClock`] used to simulate time (for unit-tests, debugging,
/// [`flashcrowd`](https://en.wiktionary.org/wiki/flashcrowd#English) simulations... etc.), it doesn't
/// go forward without explicit advancing
///
/// - [`SystemClock`] the default go-to clock, it automatically goes forward and doesn't wait around
/// 
/// For implementing clocks which can advance their time, see the extension trait 
/// [`AdvanceableScheduleClock`] for more information
///
/// **Note:** The precision of SchedulerClock can depend on the underlying OS-specific time format due
/// to the fact it uses `SystemTime` under the hood. For example, on Windows, the time is represented
/// in 100 nanosecond intervals, whereas Linux can represent nanosecond intervals... etc
/// 
/// # See
/// - [`VirtualClock`]
/// - [`SystemClock`]
/// - [`AdvanceableScheduleClock`]
#[async_trait]
pub trait SchedulerClock: Send + Sync {
    /// Gets the current time of the clock
    async fn now(&self) -> SystemTime;

    /// Idle until this specified time is reached (if it is in the past or present, it doesn't idle)
    async fn idle_to(&self, to: SystemTime);
}

/// [`AdvanceableScheduleClock`] is an optional extension to [`SchedulerClock`] which, as the name
/// suggests, allows for arbitrary advancement of time via [`AdvanceableScheduleClock::advance`] or
/// [`AdvanceableScheduleClock::advance_to`] methods, specific clocks might not support arbitrary
/// advancement (such as [`SystemClock`]), as such why it is an optional trait
/// 
/// There are also versions of these methods for converting between timezones 
/// and related time/duration types
/// 
/// # See
/// - [`SystemClock`]
/// - [`SchedulerClock`]
/// - [`VirtualClock`]
#[async_trait]
pub trait AdvanceableScheduleClock: SchedulerClock {
    /// Advance the time by a specified duration
    async fn advance(&self, duration: Duration);
        
    /// Advanced the time to a specified future time
    async fn advance_to(&self, to: SystemTime);
}

#[allow(unused)]
trait AdvanceableScheduleClockExt: AdvanceableScheduleClock {
    async fn advance<T: Into<Duration>>(&self, duration: T) {
        AdvanceableScheduleClock::advance(self, duration.into()).await;
    }

    async fn advance_timedelta(&self, duration: TimeDelta) -> Result<(), OutOfRangeError> {
        AdvanceableScheduleClock::advance(self, duration.to_std()?).await;
        Ok(())
    }

    async fn advance_to<T: TimeZone>(&self, to: DateTime<T>) {
        AdvanceableScheduleClock::advance_to(self, SystemTime::from(to)).await;
    }
}

impl<T: AdvanceableScheduleClock + ?Sized> AdvanceableScheduleClockExt for T {}