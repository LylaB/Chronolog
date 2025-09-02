use std::error::Error;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use typed_builder::TypedBuilder;
use crate::task::{Schedule, Task};

#[derive(TypedBuilder)]
#[builder(build_method(into = RetriableTask<T>))]
pub struct RetriableTaskConfig<T>
where
    T: Task,
    RetriableTask<T>: From<RetriableTaskConfig<T>>,
{
    task: T,
    retries: NonZeroU32,
    delay: Duration
}

impl<T: Task> From<RetriableTaskConfig<T>> for RetriableTask<T> {
    fn from(config: RetriableTaskConfig<T>) -> Self {
        let creation_time = Local::now();
        Self {
            task: config.task,
            retries: config.retries,
            delay: config.delay,
            last_execution: ArcSwap::from_pointee(creation_time),
        }
    }
}

pub struct RetriableTask<T: Task> {
    task: T,
    retries: NonZeroU32,
    delay: Duration,
    last_execution: ArcSwap<DateTime<Local>>
}

impl<T: Task> RetriableTask<T> {
    pub fn builder() -> RetriableTaskConfigBuilder<T> {
        RetriableTaskConfig::builder()
    }
}

#[async_trait]
impl<T: Task> Task for RetriableTask<T> {
    async fn execute(&self) -> Result<(), Arc<dyn Error + Send + Sync>> {
        let mut error: Option<Arc<dyn Error + Send + Sync>> = None;
        for _ in 0..self.retries.get() {
            let result = self.task.execute().await;
            match result {
                Ok(_) => {
                    self.last_execution.store(Arc::new(Local::now()));
                    return Ok(())
                },
                Err(err) => error = Some(err)
            }
            tokio::time::sleep(self.delay).await;
        }
        self.last_execution.store(Arc::new(Local::now()));
        Err(error.unwrap())
    }

    async fn get_schedule(&self) -> &Schedule {
        self.task.get_schedule().await
    }

    async fn total_runs(&self) -> u64 {
        self.task.total_runs().await
    }

    async fn maximum_runs(&self) -> Option<u64> {
        self.task.maximum_runs().await
    }

    async fn set_total_runs(&mut self, runs: u64) {
        self.task.set_total_runs(runs).await;
    }

    async fn get_debug_label(&self) -> String {
        self.task.get_debug_label().await
    }

    async fn last_execution(&self) -> DateTime<Local> {
        *self.last_execution.load().clone()
    }
}