pub mod execution;
pub mod sequential;
pub mod retry;
pub mod timeout;
pub mod fallback;
pub mod parallel;

use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use num_traits::Num;

#[derive(Clone)]
pub enum FieldSchedule<T: Num> {
    Ignore,
    Every(T),
    Exactly(T),
    Custom(Arc<dyn Fn(T) -> T + Send + Sync>),
}


#[derive(Clone)]
pub enum Schedule {
    Every(Duration),
    Calendar {
        year: FieldSchedule<u32>, // any value
        month: FieldSchedule<u8>, // 0 - 11
        day: FieldSchedule<u8>, // 0 - 30
        hour: FieldSchedule<u8>, // 0 - 23
        minute: FieldSchedule<u8>, // 0 - 59
        second: FieldSchedule<u8>, // 0 - 59
        millisecond: FieldSchedule<u16> // 0 - 999
    },
    Cron(String)
}

#[async_trait]
pub trait Task: Send + Sync {
    async fn execute(&self) -> Result<(), Arc<dyn Error + Send + Sync>>;

    async fn get_schedule(&self) -> &Schedule;

    async fn total_runs(&self) -> u64;
    async fn maximum_runs(&self) -> Option<u64>;

    async fn remaining_runs(&self) -> Option<u64> {
        if let Some(max_runs) = self.maximum_runs().await {
            return Some(self.total_runs().await - max_runs);
        }
        None
    }

    async fn set_total_runs(&mut self, runs: u64);

    async fn get_debug_label(&self) -> String;

    async fn last_execution(&self) -> DateTime<Local>;
}

pub enum PersistencyPolicy {
    Skip(Duration),
    RunMissed(NonZeroU32),
    RunLatest,
    RunAll
}

#[async_trait]
pub trait PersistentTask: Task {
    async fn save(&self) -> Vec<u8>;
    async fn load(&self, value: Vec<u8>) -> Arc<dyn PersistentTask + Send + Sync>;
    async fn policy(&self) -> PersistencyPolicy;
}