use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use crate::task::{TaskFrame, TaskMetadata};

/// Represents a **timeout task** which wraps a task. This task type acts as a
/// **wrapper node** within the task hierarchy, providing a timeout mechanism for execution.
///
/// ### Behavior
/// - Executes the **wrapped task**.
/// - Tracks a timer while the task executes.
/// - If the task executes longer than a specified duration, an error 
/// is thrown and the task is aborted
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
/// let retriable_task = TimeoutTask::builder()
///     .task(exec_task)
///     .max_duration(Duration::from_secs(3))
///     .build();
///
/// CHRONOLOG_SCHEDULER.register(retriable_task).await;
/// ```
pub struct TimeoutTask<T: 'static>(T, Duration);

impl<T: TaskFrame + 'static> TimeoutTask<T> {
    pub fn new(task: T, max_duration: Duration) -> Self {
        TimeoutTask(task, max_duration)
    }
}

#[async_trait]
impl<T: TaskFrame + 'static> TaskFrame for TimeoutTask<T> {
    async fn execute(&self, metadata: &(dyn TaskMetadata + Send + Sync)) -> Result<(), Arc<dyn Debug + Send + Sync>> {
        let result = tokio::time::timeout(
            self.1, self.0.execute(metadata)
        ).await;

        result.unwrap_or_else(|_| {
            Err(Arc::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "Task timed out",
            )))
        })
    }
}