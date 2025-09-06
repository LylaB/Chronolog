use std::error::Error;
use crate::task::{Arc};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use typed_builder::TypedBuilder;
use crate::errors::ChronologErrors;
use crate::task::{Schedule, Task};
use once_cell::sync::Lazy;
use crate::overlap::{OverlapStrategy, SequentialOverlapPolicy};

static EXECUTION_TASK_CREATION_COUNT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

#[derive(TypedBuilder)]
#[builder(build_method(into = ExecutionTask<F>))]
pub struct ExecutionTaskConfig<F, Fut>
where
    F: Fn(ExecutionTaskMetadata) -> Fut + Send + Sync,
    Fut: Future<Output = Result<(), Arc<dyn Error + Send + Sync>>> + Send,
    ExecutionTask<F>: From<ExecutionTaskConfig<F, Fut>>,
{
    func: F,
    
    #[builder(setter(transform = |s: impl Schedule + 'static| Arc::new(s) as Arc<dyn Schedule>))]
    schedule: Arc<dyn Schedule>,

    #[builder(default, setter(strip_option))]
    max_runs: Option<NonZeroU64>,

    #[builder(default, setter(strip_option))]
    debug_label: Option<String>,

    #[builder(default = Arc::new(SequentialOverlapPolicy), setter(transform = |s: impl OverlapStrategy + 'static| Arc::new(s) as Arc<dyn OverlapStrategy>))]
    overlap_policy: Arc<dyn OverlapStrategy>,
}

impl<F, Fut> From<ExecutionTaskConfig<F, Fut>> for ExecutionTask<F>
where
    Fut: Future<Output = Result<(), Arc<dyn Error + Send + Sync>>> + Send,
    F: Fn(ExecutionTaskMetadata) -> Fut + Send + Sync
{
    fn from(config: ExecutionTaskConfig<F, Fut>) -> Self {
        let creation_time = Local::now();
        let debug_label = if let Some(debug_label) = config.debug_label {
            debug_label
        } else {
            let num = EXECUTION_TASK_CREATION_COUNT.fetch_add(1, Ordering::Relaxed);
            format!("ExecutionTask#{}", num)
        };
        Self {
            func: config.func,
            metadata: ExecutionTaskMetadata {
                schedule: config.schedule,
                runs: 0,
                max_runs: config.max_runs,
                debug_label,
                overlap_policy: config.overlap_policy,
            },
            last_execution: ArcSwap::from_pointee(creation_time),
        }
    }
}

/// Represents a **task** that directly hosts and executes a function. This task type acts as
/// a **leaf node** within the task hierarchy. Its primary role is to serve as the final unit of
/// execution in a task workflow, as it only encapsulates a single function / future to be executed,
/// no further tasks can be chained or derived from it
///
/// ### Example
/// ```ignore
/// use std::time::Duration;
/// use chronolog::schedule::ScheduleInterval;
/// use chronolog::scheduler::{Scheduler, CHRONOLOG_SCHEDULER};
/// use chronolog::task::execution::ExecutionTask;
///
/// let task = ExecutionTask::builder()
///     .schedule(ScheduleInterval::duration(Duration::from_secs(2)))
///     .func(|_metadata| async {
///         println!("Hello from an execution task!");
///         Ok::<(), ()>(())
///     })
///     .build();
///
/// CHRONOLOG_SCHEDULER.register(task).await;
/// ```
pub struct ExecutionTask<F>
where F: Send + Sync
{
    func: F,
    metadata: ExecutionTaskMetadata,
    last_execution: ArcSwap<DateTime<Local>>,
}

/// Represents task-related metadata, it is used internally by the execution task to hold the metadata,
/// and the metadata is also exposed to the function which the execution task holds. By itself, it does
/// not allow any write operations, rather only reading its contents (internally the metadata is modified)
pub struct ExecutionTaskMetadata {
    schedule: Arc<dyn Schedule>,
    runs: u64,
    max_runs: Option<NonZeroU64>,
    debug_label: String,
    overlap_policy: Arc<dyn OverlapStrategy>
}

impl ExecutionTaskMetadata {
    /// Gets the schedule
    pub fn schedule(&self) -> Arc<dyn Schedule> {
        self.schedule.clone()
    }

    /// Gets the number of runs
    pub fn runs(&self) -> u64 {
        self.runs
    }

    /// Gets the maximum number of runs
    pub fn max_runs(&self) -> Option<NonZeroU64> {
        self.max_runs
    }

    /// Gets the human-readable debug label of the task
    pub fn debug_label(&self) -> &str {
        self.debug_label.as_str()
    }

    /// Gets the overlap task policy / strategy
    pub fn overlap_policy(&self) -> Arc<dyn OverlapStrategy> {
        self.overlap_policy.clone()
    }
}

impl<F, Fut> ExecutionTask<F>
where
    Fut: Future<Output = Result<(), Arc<dyn Error + Send + Sync>>> + Send,
    F: Fn(ExecutionTaskMetadata) -> Fut + Send + Sync
{
    /// Construct a builder for the ExecutionTask, this is a convenience method and is
    /// synonymous to:
    /// ```ignore
    /// ExecutionTaskConfig::<F, _>::builder()
    /// ```
    pub fn builder() -> ExecutionTaskConfigBuilder<F, Fut> {
        ExecutionTaskConfig::builder()
    }
}

#[async_trait]
impl<F, Fut> Task for ExecutionTask<F>
where
    Fut: Future<Output = Result<(), Arc<dyn Error + Send + Sync>>> + Send,
    F: Fn(ExecutionTaskMetadata) -> Fut + Send + Sync
{
    async fn execute_inner(&self) -> Result<(), Arc<dyn Error + Send + Sync>> {
        let cloned_metadata = ExecutionTaskMetadata {
            schedule: self.metadata.schedule.clone(),
            runs: self.metadata.runs,
            max_runs: self.metadata.max_runs,
            overlap_policy: self.metadata.overlap_policy.clone(),
            debug_label: self.metadata.debug_label.clone(),
        };
        let result = (self.func)(cloned_metadata).await;
        if let Err(err) = result {
            return Err(Arc::new(ChronologErrors::FailedExecution(
                self.get_debug_label().await, Box::new(err))
            ));
        }
        self.last_execution.swap(Arc::new(Local::now()));
        Ok(())
    }

    async fn get_schedule(&self) -> Arc<dyn Schedule> {
        self.metadata.schedule.clone()
    }

    async fn total_runs(&self) -> u64 {
        self.metadata.runs
    }

    async fn maximum_runs(&self) -> Option<NonZeroU64> {
        self.metadata.max_runs
    }

    async fn set_maximum_runs(&mut self, max_runs: NonZeroU64) {
        self.metadata.max_runs = Some(max_runs)
    }

    async fn set_total_runs(&mut self, runs: u64) {
        self.metadata.runs = runs;
    }

    async fn set_last_execution(&mut self, exec: DateTime<Local>) {
        self.last_execution.swap(Arc::new(exec));
    }

    async fn get_debug_label(&self) -> String {
        self.metadata.debug_label.clone()
    }

    async fn last_execution(&self) -> DateTime<Local> {
        *self.last_execution.load().clone()
    }

    async fn overlap_policy(&self) -> Arc<dyn OverlapStrategy> {
        self.metadata.overlap_policy.clone()
    }
}