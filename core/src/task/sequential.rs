use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;
use crate::task::{TaskFrame, TaskMetadata};
use crate::task::parallel::ParallelTask;

pub enum SequentialTaskPolicy {
    RunSilenceFailures,
    RunUntilSuccess,
    RunUntilFailure,
}

/// Represents a **sequential task** which wraps multiple tasks to execute at the same time in a sequential
/// manner. This task type acts as a **composite node** within the task hierarchy, facilitating a way to
/// represent multiple tasks which have same timings but depend on each previous task finishing. The
/// order of execution is ordered, and thus why its sequential, in the case where execution order
/// does not matter, tasks do not require sequential execution, it is advised to use [`ParallelTask`] 
/// as opposed to [`SequentialTask`]
///
/// # Example
/// ```ignore
/// use std::sync::Arc;
/// use std::time::Duration;
/// use chronolog::schedule::ScheduleInterval;
/// use chronolog::scheduler::{Scheduler, CHRONOLOG_SCHEDULER};
/// use chronolog::task::fallback::FallbackTask;
/// use chronolog::task::execution::ExecutionTask;
/// use chronolog::task::parallel::SequentialTask;
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
/// let sequential_task = SequentialTask::builder()
///     .tasks(vec![Arc::new(primary_task), Arc::new(secondary_task), Arc::new(tertiary_task)])
///     .build();
///
/// CHRONOLOG_SCHEDULER.register(sequential_task).await;
/// ```
pub struct SequentialTask(Vec<Arc<dyn TaskFrame>>, SequentialTaskPolicy);

impl SequentialTask {
    pub fn new(tasks: Vec<Arc<dyn TaskFrame>>) -> SequentialTask {
        SequentialTask(tasks, SequentialTaskPolicy::RunSilenceFailures)
    }
    
    pub fn new_with(tasks: Vec<Arc<dyn TaskFrame>>, policy: SequentialTaskPolicy) -> SequentialTask {
        SequentialTask(tasks, policy)
    }
}

#[async_trait]
impl TaskFrame for SequentialTask {
    async fn execute(&self, metadata: &(dyn TaskMetadata + Send + Sync)) -> Result<(), Arc<dyn Debug + Send + Sync>> {
        for task in self.0.iter() {
            let result = task.execute(metadata).await;
            match (&self.1, result) {
                (SequentialTaskPolicy::RunUntilFailure, Err(res)) => {
                    return Err(res)
                }
                (SequentialTaskPolicy::RunUntilSuccess, Ok(res)) => {
                    return Ok(res);
                }
                (_, Ok(_)) => {}
                (_, Err(_)) => {}
            }
        }
        Ok(())
    }
}