use crate::task::{ArcTaskEvent, ExposedTaskMetadata, TaskEndEvent, TaskError, TaskEvent, TaskStartEvent};
use std::fmt::Debug;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use crate::task::{TaskEventEmitter, TaskFrame};

/// Represents a **retriable task frame** which wraps a task frame. This task frame type acts as a
/// **wrapper node** within the task frame hierarchy, providing a retry mechanism for execution.
///
/// # Behavior
/// - Executes the **wrapped task frame**.
/// - If the task frame fails, it re-executes it again after a specified delay (or instantaneous).
/// - Repeat the process for a specified number of retries til the task frame succeeds
///
/// # Events
/// [`RetriableTaskFrame`] provides 2 events, namely ``on_retry_start`` which executes when a retry
/// happens, it hands out the wrapped task frame instance. As well as the ``on_retry_end`` which 
/// executes when a retry is finished, it hands out the wrapped task frame instance and an option 
/// error for a potential error it may have gotten from this retry
///
/// # Example
/// ```ignore
/// use std::num::NonZeroU32;
/// use chronolog_core::schedule::TaskScheduleInterval;
/// use chronolog_core::scheduler::{Scheduler, CHRONOLOG_SCHEDULER};
/// use chronolog_core::task::retry::RetriableTaskFrame;
/// use chronolog_core::task::execution::ExecutionTaskFrame;
/// use chronolog_core::task::Task;
///
/// let exec_frame = ExecutionTaskFrame::new(
///     |_metadata| async {
///         println!("Trying primary task...");
///         Err::<(), ()>(())
///     }
/// );
///
/// let retriable_frame = RetriableTaskFrame::new_instant(
///     exec_frame,
///     NonZeroU32::new(3).unwrap(), // We know it isn't zero, so safe to unwrap
/// );
///
/// let task = Task::define(TaskScheduleInterval::from_secs_f64(2.5), retriable_frame);
///
/// CHRONOLOG_SCHEDULER.register(task).await;
/// ```
pub struct RetriableTaskFrame<T: 'static> {
    task: Arc<T>,
    retries: NonZeroU32,
    delay: Duration,
    on_start: TaskStartEvent,
    on_end: TaskEndEvent,
    pub on_retry_start: ArcTaskEvent<Arc<T>>,
    pub on_retry_end: ArcTaskEvent<(Arc<T>, Option<TaskError>)>,
}

impl<T: TaskFrame + 'static> RetriableTaskFrame<T> {
    /// Creates a retriable task that has a specified delay per retry
    pub fn new(task: T, retries: NonZeroU32, delay: Duration) -> Self {
        Self {
            task: Arc::new(task), 
            retries, 
            delay,
            on_start: TaskEvent::new(),
            on_end: TaskEvent::new(),
            on_retry_end: TaskEvent::new(),
            on_retry_start: TaskEvent::new()
        }
    }

    /// Creates a retriable task that has no delay per retry
    pub fn new_instant(task: T, retries: NonZeroU32) -> Self {
        Self::new(task, retries, Duration::ZERO)
    }
}

#[async_trait]
impl<T: TaskFrame + 'static> TaskFrame for RetriableTaskFrame<T> {
    async fn execute(
        &self,
        metadata: Arc<dyn ExposedTaskMetadata + Send + Sync>,
        emitter: Arc<TaskEventEmitter>
    ) -> Result<(), TaskError> {
        let mut error: Option<TaskError> = None;
        for i in 0..self.retries.get() {
            if i != 0 {
                emitter.emit(metadata.clone(), self.on_retry_start.clone(), self.task.clone()).await;
            }
            let result = self.task.execute(metadata.clone(), emitter.clone()).await;
            match result {
                Ok(_) => {
                    emitter.emit(metadata.clone(), self.on_retry_end.clone(), (self.task.clone(), None)).await;
                    return Ok(())
                },
                Err(err) => {
                    error = Some(err.clone());
                    emitter.emit(metadata.clone(), self.on_retry_end.clone(), (self.task.clone(), error.clone())).await;
                }
            }
            tokio::time::sleep(self.delay).await;
        }
        Err(error.unwrap())
    }

    fn on_start(&self) -> TaskStartEvent {
        self.on_start.clone()
    }

    fn on_end(&self) -> TaskEndEvent {
        self.on_end.clone()
    }
}