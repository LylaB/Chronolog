use std::time::{Duration, SystemTime};
use async_trait::async_trait;
use crate::clock::SchedulerClock;

#[allow(unused_imports)]
use crate::clock::VirtualClock;

/// [`SystemClock`] is an implementation of [`SchedulerClock`] trait, it is the default option
/// for scheduling, unlike [`VirtualClock`], it moves forward no matter what and cannot be advanced
/// at any arbitrary point (due to its design)
///
/// # See
/// - [`VirtualClock`]
/// - [`SchedulerClock`]
pub struct SystemClock;

#[async_trait]
impl SchedulerClock for SystemClock {
    async fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    async fn idle_to(&self, to: SystemTime) {
        let now = SystemTime::now();
        let duration = match to.duration_since(now) {
            Ok(duration) => duration,
            Err(diff) => {
                if diff.duration() <= Duration::from_millis(7) {
                    return;
                }
                panic!("Supposed future time is now in the past with a difference of {:?}", diff.duration());
            }
        };

        tokio::time::sleep(duration).await;
    }
}