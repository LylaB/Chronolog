use std::time::{SystemTime, UNIX_EPOCH};
use async_trait::async_trait;
use crate::clock::{SchedulerClock, VirtualClock};

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
        tokio::time::sleep(to.duration_since(UNIX_EPOCH).unwrap()).await;
    }
}