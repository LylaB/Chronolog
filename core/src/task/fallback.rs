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
#[builder(build_method(into = FallbackTask<T, T2>))]
pub struct FallbackTaskConfig<T, T2>
where
    T: Task,
    T2: Task,
    FallbackTask<T, T2>: From<FallbackTaskConfig<T, T2>>,
{
    primary: T,
    fallback_task: T2,

    #[builder(default = Arc::new(SequentialOverlapPolicy))]
    overlap_policy: Arc<dyn OverlapStrategy>,

    #[builder(default, setter(strip_option))]
    max_runs: Option<NonZeroU64>,
}

impl<T: Task, T2: Task> From<FallbackTaskConfig<T, T2>> for FallbackTask<T, T2> {
    fn from(config: FallbackTaskConfig<T, T2>) -> Self {
        let creation_time = Local::now();
        Self {
            task: config.primary,
            fallback: config.fallback_task,
            max_runs: config.max_runs,
            last_execution: ArcSwap::from_pointee(creation_time),
            overlap_policy: config.overlap_policy,
        }
    }
}

/// Represents a **fallback task** which wraps two other tasks. This task type acts as a
/// **composite node** within the task hierarchy, providing a failover mechanism for execution.
///
/// ### Behavior
/// - Executes the **primary task** first.
/// - If the primary task completes successfully, the fallback task is **skipped**.
/// - If the primary task **fails**, the **secondary task** is executed as a fallback.
///
/// # Example
/// ```ignore
/// use std::time::Duration;
/// use chronolog::schedule::ScheduleInterval;
/// use chronolog::scheduler::{Scheduler, CHRONOLOG_SCHEDULER};
/// use chronolog::task::fallback::FallbackTask;
/// use chronolog::task::execution::ExecutionTask;
///
/// let primary_task = ExecutionTask::builder()
///     .schedule(ScheduleInterval::duration(Duration::from_secs(2)))
///     .func(|_metadata| async {
///         println!("Trying primary task...");
///         Err::<(), ()>(())
///     })
///     .build();
///
/// let secondary_task = ExecutionTask::builder()
///     .schedule(ScheduleInterval::duration(Duration::from_secs(2)))
///     .func(|_metadata| async {
///         println!("Primary failed, running fallback task!");
///         Ok::<(), ()>(())
///     })
///     .build();
///
/// let fallback_task = FallbackTask::builder()
///     .primary(primary_task)
///     .fallback(secondary_task)
///     .build();
///
/// CHRONOLOG_SCHEDULER.register(fallback_task).await;
/// ```
pub struct FallbackTask<T: Task, T2: Task> {
    task: T,
    fallback: T2,
    last_execution: ArcSwap<DateTime<Local>>,
    max_runs: Option<NonZeroU64>,
    overlap_policy: Arc<dyn OverlapStrategy>,
}

impl<T: Task, T2: Task> FallbackTask<T, T2> {
    pub fn builder() -> FallbackTaskConfigBuilder<T, T2> {
        FallbackTaskConfig::builder()
    }
}

#[async_trait]
impl<T: Task, T2: Task> Task for FallbackTask<T, T2> {
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