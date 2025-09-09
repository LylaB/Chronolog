use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;
use crate::task::{TaskFrame, TaskMetadata};

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
pub struct FallbackTask<T: 'static, T2: 'static>(T, T2);

impl<T, T2> FallbackTask<T, T2>
where
    T: TaskFrame + 'static,
    T2: TaskFrame + 'static,
{
    pub fn new(primary: T, secondary: T2) -> Self {
        Self(primary, secondary)
    }
}

#[async_trait]
impl<T, T2> TaskFrame for FallbackTask<T, T2>
where
    T: TaskFrame + 'static,
    T2: TaskFrame + 'static,
{
    async fn execute(&self, metadata: &(dyn TaskMetadata + Send + Sync)) -> Result<(), Arc<dyn Debug + Send + Sync>> {
        let result = match self.0.execute(metadata).await {
            Err(_) => {
                let result = self.1.execute(metadata).await;
                result
            },
            res => res
        };
        result
    }
}