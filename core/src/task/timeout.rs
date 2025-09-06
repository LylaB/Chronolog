use std::error::Error;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::time::Duration;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use typed_builder::TypedBuilder;
use crate::overlap::OverlapStrategy;
use crate::task::{Schedule, Task};

#[derive(TypedBuilder)]
#[builder(build_method(into = TimeoutTask<T>))]
pub struct TimeoutTaskConfig<T>
where
    T: Task,
    TimeoutTask<T>: From<TimeoutTaskConfig<T>>,
{
    task: T,
    max_duration: Duration
}

impl<T: Task> From<TimeoutTaskConfig<T>> for TimeoutTask<T> {
    fn from(config: TimeoutTaskConfig<T>) -> Self {
        let creation_time = Local::now();
        Self {
            task: config.task,
            max_duration: config.max_duration,
            last_execution: ArcSwap::new(Arc::new(creation_time)),
        }
    }
}

pub struct TimeoutTask<T: Task> {
    task: T,
    max_duration: Duration,
    last_execution: ArcSwap<DateTime<Local>>
}

impl<T: Task> TimeoutTask<T> {
    pub fn builder() -> TimeoutTaskConfigBuilder<T> {
        TimeoutTaskConfig::builder()
    }
}

#[async_trait]
impl<T: Task> Task for TimeoutTask<T> {
    async fn execute_inner(&self) -> Result<(), Arc<dyn Error + Send + Sync>> {
        tokio::time::timeout(self.max_duration, self.task.execute_inner())
            .await
            .unwrap_or_else(|_| Err(Arc::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Task timed out",
            ))))
    }

    async fn get_schedule(&self) -> Arc<dyn Schedule> {
        self.task.get_schedule().await
    }

    async fn total_runs(&self) -> u64 {
        self.task.total_runs().await
    }

    async fn maximum_runs(&self) -> Option<NonZeroU64> {
        self.task.maximum_runs().await
    }

    async fn set_maximum_runs(&mut self, max_runs: NonZeroU64) {
        self.task.set_maximum_runs(max_runs).await;
    }

    async fn set_total_runs(&mut self, runs: u64) {
        self.task.set_total_runs(runs).await;
    }

    async fn set_last_execution(&mut self, exec: DateTime<Local>) {
        self.last_execution.swap(Arc::new(exec));
    }

    async fn get_debug_label(&self) -> String {
        self.task.get_debug_label().await
    }

    async fn last_execution(&self) -> DateTime<Local> {
        *self.last_execution.load().clone()
    }

    async fn overlap_policy(&self) -> Arc<dyn OverlapStrategy> {
        self.task.overlap_policy().await
    }
}