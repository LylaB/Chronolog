use std::error::Error;
use std::sync::Arc;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use typed_builder::TypedBuilder;
use crate::task::{Schedule, Task};

#[derive(TypedBuilder)]
#[builder(build_method(into = FallbackTask<T>))]
pub struct FallbackTaskConfig<T>
where
    T: Task,
    FallbackTask<T>: From<FallbackTaskConfig<T>>,
{
    task: T,
    fallback_task: T
}

impl<T: Task> From<FallbackTaskConfig<T>> for FallbackTask<T> {
    fn from(config: FallbackTaskConfig<T>) -> Self {
        let creation_time = Local::now();
        Self {
            task: config.task,
            fallback: config.fallback_task,
            last_execution: ArcSwap::from_pointee(creation_time),
        }
    }
}

pub struct FallbackTask<T: Task> {
    task: T,
    fallback: T,
    last_execution: ArcSwap<DateTime<Local>>,
}

impl<T: Task> FallbackTask<T> {
    pub fn builder() -> FallbackTaskConfigBuilder<T> {
        FallbackTaskConfig::builder()
    }
}

#[async_trait]
impl<T: Task> Task for FallbackTask<T> {
    async fn execute(&self) -> Result<(), Arc<dyn Error + Send + Sync>> {
        let result = match self.task.execute().await {
            Err(_) => self.fallback.execute().await,
            res => res
        };
        self.last_execution.store(Arc::new(Local::now()));
        result
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

    async fn set_total_runs(&mut self, _runs: u64) {}

    async fn get_debug_label(&self) -> String {
        self.task.get_debug_label().await
    }

    async fn last_execution(&self) -> DateTime<Local> {
        *self.last_execution.load().clone()
    }
}