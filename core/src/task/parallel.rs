use std::error::Error;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use typed_builder::TypedBuilder;
use crate::task::{Schedule, Task, sequential::SequentialTask};
use once_cell::sync::Lazy;
use crate::overlap::{OverlapStrategy, SequentialOverlapPolicy};

static PARALLEL_TASK_CREATION_COUNT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

pub enum ParallelTaskPolicy {
    RunSilenceFailures,
    RunUntilSuccess,
    RunUntilFailure,
}

#[derive(TypedBuilder)]
#[builder(build_method(into = ParallelTask))]
pub struct ParallelTaskConfig
where ParallelTask: From<ParallelTaskConfig> {
    tasks: Vec<Arc<dyn Task>>,

    #[builder(setter(transform = |s: impl Schedule + 'static| Arc::new(s) as Arc<dyn Schedule>))]
    schedule: Arc<dyn Schedule>,

    #[builder(default, setter(strip_option))]
    max_runs: Option<NonZeroU64>,

    #[builder(default, setter(strip_option))]
    debug_label: Option<String>,

    #[builder(default = ParallelTaskPolicy::RunUntilFailure)]
    policy: ParallelTaskPolicy,

    #[builder(default = Arc::new(SequentialOverlapPolicy), setter(transform = |s: impl OverlapStrategy + 'static| Arc::new(s) as Arc<dyn OverlapStrategy>))]
    overlap_policy: Arc<dyn OverlapStrategy>,
}

impl From<ParallelTaskConfig> for ParallelTask {
    fn from(config: ParallelTaskConfig) -> Self {
        let creation_time = Local::now();
        let debug_label = if let Some(debug_label) = config.debug_label {
            debug_label
        } else {
            let num = PARALLEL_TASK_CREATION_COUNT.fetch_add(1, Ordering::Relaxed);
            format!("ParallelTask#{}", num)
        };
        Self {
            tasks: config.tasks,
            schedule: config.schedule,
            runs: 0,
            max_runs: config.max_runs,
            debug_label,
            parallel_policy: config.policy,
            overlap_policy: config.overlap_policy,
            last_execution: ArcSwap::from_pointee(creation_time)
        }
    }
}

/// Represents a **parallel task** which wraps multiple tasks to execute at the same time. This task type
/// acts as a **composite node** within the task hierarchy, facilitating a way to represent multiple tasks
/// which have same timings. This is much more optimized than dispatching those tasks on the scheduler
/// independently, each individual task's schedule is ignored, instead, a group schedule is used. The
/// order of execution is unordered, and thus one task may be executed sooner than another, in this case,
/// it is advised to use [`SequentialTask`] as opposed to [`ParallelTask`]
///
/// # Example
/// ```ignore
/// use std::sync::Arc;
/// use std::time::Duration;
/// use chronolog::schedule::ScheduleInterval;
/// use chronolog::scheduler::{Scheduler, CHRONOLOG_SCHEDULER};
/// use chronolog::task::fallback::FallbackTask;
/// use chronolog::task::execution::ExecutionTask;
/// use chronolog::task::parallel::ParallelTask;
///
/// let primary_task = ExecutionTask::builder()
///     .schedule(ScheduleInterval::default())
///     .func(|_metadata| async {
///         println!("Executing Primary Task");
///         Ok(())
///     })
///     .build();
///
/// let secondary_task = ExecutionTask::builder()
///     .schedule(ScheduleInterval::default())
///     .func(|_metadata| async {
///         println!("Executing Secondary Task");
///         Ok(())
///     })
///     .build();
///
/// let tertiary_task = ExecutionTask::builder()
///     .schedule(ScheduleInterval::default())
///     .func(|_metadata| async {
///         println!("Executing Tertiary Task");
///         Err(())
///     })
///     .build();
///
/// let parallel_task = ParallelTask::builder()
///     .tasks(vec![Arc::new(primary_task), Arc::new(secondary_task), Arc::new(tertiary_task)])
///     .build();
///
/// CHRONOLOG_SCHEDULER.register(parallel_task).await;
/// ```
pub struct ParallelTask {
    tasks: Vec<Arc<dyn Task>>,
    schedule: Arc<dyn Schedule>,
    runs: u64,
    max_runs: Option<NonZeroU64>,
    debug_label: String,
    parallel_policy: ParallelTaskPolicy,
    overlap_policy: Arc<dyn OverlapStrategy>,
    last_execution: ArcSwap<DateTime<Local>>
}

impl ParallelTask {
    pub fn builder() -> ParallelTaskConfigBuilder {
        ParallelTaskConfig::builder()
    }
}

#[async_trait]
impl Task for ParallelTask {
    async fn execute_inner(&self) -> Result<(), Arc<dyn Error + Send + Sync>> {
        let mut futures = FuturesUnordered::from_iter(
            self.tasks.iter().map(|x| x.execute_inner())
        );

        while let Some(result) = futures.next().await {
            match (&self.parallel_policy, result) {
                (ParallelTaskPolicy::RunUntilFailure, Err(res)) => {
                    return Err(res)
                }
                (ParallelTaskPolicy::RunUntilSuccess, Ok(res)) => {
                    return Ok(res);
                }
                (_, Ok(_)) => {}
                (_, Err(_)) => {}
            }
        }
        self.last_execution.store(Arc::new(Local::now()));
        Ok(())
    }

    async fn get_schedule(&self) -> Arc<dyn Schedule> {
        self.schedule.clone()
    }

    async fn total_runs(&self) -> u64 {
        self.runs
    }

    async fn maximum_runs(&self) -> Option<NonZeroU64> {
        self.max_runs
    }

    async fn set_maximum_runs(&mut self, max_runs: NonZeroU64) {
        self.max_runs = Some(max_runs)
    }
    
    async fn set_total_runs(&mut self, runs: u64) {
        self.runs = runs;
    }

    async fn set_last_execution(&mut self, exec: DateTime<Local>) {
        self.last_execution.swap(Arc::new(exec));
    }

    async fn get_debug_label(&self) -> String {
        self.debug_label.clone()
    }

    async fn last_execution(&self) -> DateTime<Local> {
        *self.last_execution.load().clone()
    }

    async fn overlap_policy(&self) -> Arc<dyn OverlapStrategy> {
        self.overlap_policy.clone()
    }
}