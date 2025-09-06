use std::error::Error;
use std::num::NonZeroU64;
use std::sync::Arc;
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use typed_builder::TypedBuilder;
use crate::overlap::{OverlapStrategy, SequentialOverlapPolicy};
use crate::task::{Schedule, Task};

#[derive(TypedBuilder)]
#[builder(build_method(into = FallbackTask<T>))]
pub struct FallbackTaskConfig<T>
where
    T: Task,
    FallbackTask<T>: From<FallbackTaskConfig<T>>,
{
    task: T,
    fallback_task: T,

    #[builder(default = Arc::new(SequentialOverlapPolicy))]
    overlap_policy: Arc<dyn OverlapStrategy>,

    #[builder(default, setter(strip_option))]
    max_runs: Option<NonZeroU64>,
}

impl<T: Task> From<FallbackTaskConfig<T>> for FallbackTask<T> {
    fn from(config: FallbackTaskConfig<T>) -> Self {
        let creation_time = Local::now();
        Self {
            task: config.task,
            fallback: config.fallback_task,
            max_runs: config.max_runs,
            last_execution: ArcSwap::from_pointee(creation_time),
            overlap_policy: config.overlap_policy,
        }
    }
}

pub struct FallbackTask<T: Task> {
    task: T,
    fallback: T,
    last_execution: ArcSwap<DateTime<Local>>,
    max_runs: Option<NonZeroU64>,
    overlap_policy: Arc<dyn OverlapStrategy>,
}

impl<T: Task> FallbackTask<T> {
    pub fn builder() -> FallbackTaskConfigBuilder<T> {
        FallbackTaskConfig::builder()
    }
}

#[async_trait]
impl<T: Task> Task for FallbackTask<T> {
    async fn execute_inner(&self) -> Result<(), Arc<dyn Error + Send + Sync>> {
        let result = match self.task.execute_inner().await {
            Err(_) => self.fallback.execute_inner().await,
            res => res
        };
        self.last_execution.store(Arc::new(Local::now()));
        result
    }

    async fn get_schedule(&self) -> Arc<dyn Schedule> {
        self.task.get_schedule().await
    }

    async fn total_runs(&self) -> u64 {
        self.task.total_runs().await
    }

    async fn maximum_runs(&self) -> Option<NonZeroU64> {
        self.max_runs
    }

    async fn set_maximum_runs(&mut self, max_runs: NonZeroU64) {
        self.max_runs = Some(max_runs)
    }

    async fn set_total_runs(&mut self, _runs: u64) {}

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
        self.overlap_policy.clone()
    }
}