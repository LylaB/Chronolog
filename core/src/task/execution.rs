use std::fmt::Debug;
use crate::task::{Arc, TaskFrame, TaskMetadata};
use async_trait::async_trait;

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
///         Ok(())
///     })
///     .build();
///
/// CHRONOLOG_SCHEDULER.register(task).await;
/// ```
pub struct ExecutionTaskFrame<F: Send + Sync>(F);

impl<F, Fut> ExecutionTaskFrame<F>
where
    Fut: Future<Output = Result<(), Arc<dyn Debug + Send + Sync>>> + Send,
    F: Fn(&dyn TaskMetadata) -> Fut + Send + Sync
{
    pub fn new(func: F) -> Self {
        ExecutionTaskFrame(func)
    }
}

#[async_trait]
impl<F, Fut> TaskFrame for ExecutionTaskFrame<F>
where
    Fut: Future<Output = Result<(), Arc<dyn Debug + Send + Sync>>> + Send,
    F: Fn(&dyn TaskMetadata) -> Fut + Send + Sync
{
    async fn execute(&self, metadata: &(dyn TaskMetadata + Send + Sync)) -> Result<(), Arc<dyn Debug + Send + Sync>> {
        self.0(metadata).await
    }
}