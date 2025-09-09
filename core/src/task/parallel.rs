use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use crate::task::{sequential::SequentialTask, TaskFrame, TaskMetadata};

pub enum ParallelTaskPolicy {
    RunSilenceFailures,
    RunUntilSuccess,
    RunUntilFailure,
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
pub struct ParallelTask(Vec<Arc<dyn TaskFrame>>, ParallelTaskPolicy);

impl ParallelTask {
    pub fn new(tasks: Vec<Arc<dyn TaskFrame>>) -> Self {
        Self(tasks, ParallelTaskPolicy::RunSilenceFailures)
    }

    pub fn new_with(tasks: Vec<Arc<dyn TaskFrame>>, parallel_task_policy: ParallelTaskPolicy) -> Self {
        Self(tasks, parallel_task_policy)
    }
}

#[async_trait]
impl TaskFrame for ParallelTask {
    async fn execute(&self, metadata: &(dyn TaskMetadata + Send + Sync)) -> Result<(), Arc<dyn Debug + Send + Sync>> {
        let mut futures = FuturesUnordered::from_iter(
            self.0.iter().map(|x| {
                async move {
                    let result = x.execute(metadata).await;
                    result
                }
            })
        );

        while let Some(result) = futures.next().await {
            match (&self.1, result) {
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
        Ok(())
    }
}