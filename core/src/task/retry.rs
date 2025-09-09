use std::fmt::Debug;
use std::num::{NonZeroU32};
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use crate::task::{TaskFrame, TaskMetadata};

/// Represents a **retriable task** which wraps a task. This task type acts as a
/// **wrapper node** within the task hierarchy, providing a retry mechanism for execution.
///
/// ### Behavior
/// - Executes the **wrapped task**.
/// - If the task fails, it re-executes it again after a specified delay (or instantaneous).
/// - Repeat the process for a specified number of retries til the task succeeds
///
/// # Example
/// ```ignore
/// use std::time::Duration;
/// use chronolog::schedule::ScheduleInterval;
/// use chronolog::scheduler::{Scheduler, CHRONOLOG_SCHEDULER};
/// use chronolog::task::fallback::FallbackTask;
/// use chronolog::task::execution::ExecutionTask;
///
/// let exec_task = ExecutionTask::builder()
///     .schedule(ScheduleInterval::duration(Duration::from_secs(2)))
///     .func(|_metadata| async {
///         println!("Trying primary task...");
///         Err::<(), ()>(())
///     })
///     .build();
///
/// let retriable_task = RetriableTask::builder()
///     .task(exec_task)
///     .retries(3)
///     .delay(Duration::from_secs(0))
///     .build();
///
/// CHRONOLOG_SCHEDULER.register(retriable_task).await;
/// ```
pub struct RetriableTask<T: 'static> {
    task: T,
    retries: NonZeroU32,
    delay: Duration,
}

impl<T: TaskFrame + 'static> RetriableTask<T> {
    pub fn new(task: T, retries: NonZeroU32, delay: Duration) -> Self {
        Self {task, retries, delay}
    }

    pub fn new_instant(task: T, retries: NonZeroU32) -> Self {
        Self {task, retries, delay: Duration::ZERO}
    }
}

#[async_trait]
impl<T: TaskFrame + 'static> TaskFrame for RetriableTask<T> {
    async fn execute(&self, metadata: &(dyn TaskMetadata + Send + Sync)) -> Result<(), Arc<dyn Debug + Send + Sync>> {
        let mut error: Option<Arc<dyn Debug + Send + Sync>> = None;
        for _ in 0..self.retries.get() {
            let result = self.task.execute(metadata).await;
            match result {
                Ok(_) => {
                    return Ok(())
                },
                Err(err) => {
                    error = Some(err.clone());
                }
            }
            tokio::time::sleep(self.delay).await;
        }
        Err(error.unwrap())
    }
}