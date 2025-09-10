use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use arc_swap::ArcSwapOption;
use chrono::{DateTime, Local};
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;
use crate::task::{Task, TaskErrorContext, TaskEventEmitter};

/// [`ScheduleStrategy`] defines how the task behaves when being overlapped by the same instance
/// task or by others. Strategies receive a task, an event emitter and return a local timestamp
/// indicating the last time it was executed. It is their duty for strategies to correctly
/// call the lifecycle events, handle the run count and call the error handler (all of this is
/// automatically in [`<dyn ScheduleStrategy>::execute_logic`] for convenience’s sake
#[async_trait::async_trait]
pub trait ScheduleStrategy: Send + Sync {
    async fn handle(&self, task: Arc<Task>, emitter: Arc<TaskEventEmitter>) -> DateTime<Local>;
}

impl dyn ScheduleStrategy {
    async fn execute_logic(emitter: Arc<TaskEventEmitter>, task: Arc<Task>) {
        task.metadata.runs().fetch_add(1, Ordering::Relaxed);
        emitter.emit(task.metadata (), task.frame().on_start(), ()).await;
        let result = task.frame().execute(task.metadata.as_exposed(), emitter.clone()).await;
        let err = result.err();
        emitter.emit(task.metadata (), task.frame().on_end(), err.clone()).await;
        if let Some(error) = err {
            let error_ctx = TaskErrorContext {
                error,
                metadata: task.metadata.as_exposed().clone()
            };

            task.error_handler().on_error(error_ctx).await;
        }
    }
}

/// [`SequentialSchedulingPolicy`] is an implementation of [`ScheduleStrategy`] and executes the
/// task sequentially, only once it finishes, does it reschedule the same task. This is the default
/// scheduling strategy used by [`Task`]
pub struct SequentialSchedulingPolicy;
#[async_trait::async_trait]
impl ScheduleStrategy for SequentialSchedulingPolicy {
    async fn handle(&self, task: Arc<Task>, emitter: Arc<TaskEventEmitter>) -> DateTime<Local> {
        <dyn ScheduleStrategy>::execute_logic(emitter, task).await;
        Local::now()
    }
}

/// [`ConcurrentSchedulingPolicy`] is an implementation of [`ScheduleStrategy`] and executes the task
/// in the background, while it also reschedules other tasks to execute, one should be careful when
/// using this to not run into the [Thundering Herd Problem](https://en.wikipedia.org/wiki/Thundering_herd_problem)
pub struct ConcurrentSchedulingPolicy;
#[async_trait::async_trait]
impl ScheduleStrategy for ConcurrentSchedulingPolicy {
    async fn handle(&self, task: Arc<Task>, emitter: Arc<TaskEventEmitter>) -> DateTime<Local> {
        tokio::spawn(async move {
            <dyn ScheduleStrategy>::execute_logic(emitter, task).await;
        });
        Local::now()
    }
}

/// [`CancelPreviousSchedulingPolicy`] is an implementation of [`ScheduleStrategy`] and executes the
/// task in the background, unlike [`ConcurrentSchedulingPolicy`], this policy cancels the previous
/// task process if a new task overlaps it
///
/// **⚠ Note ⚠** due to a limitation, if the task frame executes CPU-Bound logic mostly and does not yield,
/// then the task frame may be completed, as such ensure the task frame has defined a sufficient
/// number of cancellation points / yields
pub struct CancelPreviousSchedulingPolicy {
    process: ArcSwapOption<JoinHandle<()>>,
    cancel_tx: Mutex<Option<watch::Sender<()>>>,
}

impl CancelPreviousSchedulingPolicy {
    pub fn new() -> Self {
        Self {
            process: ArcSwapOption::new(None),
            cancel_tx: Mutex::new(None),
        }
    }
}

#[async_trait::async_trait]
impl ScheduleStrategy for CancelPreviousSchedulingPolicy {
    async fn handle(&self, task: Arc<Task>, emitter: Arc<TaskEventEmitter>) -> DateTime<Local> {
        emitter.emit(task.metadata (), task.frame().on_start(), ()).await;
        let old_handle = self.process.swap(None);
        let mut cancel_lock = self.cancel_tx.lock().await;
        let old_cancel_tx = cancel_lock.take();
        drop(cancel_lock);

        if let (Some(handle), Some(cancel_tx)) = (old_handle, old_cancel_tx) {
            let _ = cancel_tx.send(());
            handle.abort();
        }

        let (cancel_tx, mut cancel_rx) = watch::channel(());

        let handle = tokio::spawn(async move {
            if cancel_rx.has_changed().unwrap() {
                return;
            }

            let cancel_rx_clone = cancel_rx.clone();

            tokio::select! {
                _ = async {
                    emitter.emit(task.metadata (), task.frame().on_start(), ()).await;

                    let mut interval = tokio::time::interval(Duration::from_millis(100));
                    let frame = task.frame();

                    tokio::select! {
                            _ = interval.tick() => {
                                if cancel_rx_clone.has_changed().unwrap() {
                                    return;
                                }
                            }

                            result = frame.execute(task.metadata.as_exposed(), emitter.clone()) => {
                                let err = result.err();
                                emitter.emit(task.metadata (), task.frame().on_end(), err.clone()).await;
                                if let Some(error) = err {
                                    let error_ctx = TaskErrorContext {
                                        error,
                                        metadata: task.metadata.as_exposed().clone()
                                    };

                                    task.error_handler().on_error(error_ctx).await;
                                }
                            }
                        }
                } => {}

                _ = cancel_rx.changed() => {}
            }
        });

        self.process.store(Some(Arc::new(handle)));
        let mut cancel_lock = self.cancel_tx.lock().await;
        *cancel_lock = Some(cancel_tx);
        Local::now()
    }
}

/// [`CancelCurrentSchedulingPolicy`] is an implementation of [`ScheduleStrategy`] and executes the
/// task in the background, unlike [`ConcurrentSchedulingPolicy`], this policy cancels the current
/// task that tries to overlaps the already-running task
pub struct CancelCurrentSchedulingPolicy(Arc<AtomicBool>);

impl CancelCurrentSchedulingPolicy {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }
}

#[async_trait::async_trait]
impl ScheduleStrategy for CancelCurrentSchedulingPolicy {
    async fn handle(&self, task: Arc<Task>, emitter: Arc<TaskEventEmitter>) -> DateTime<Local> {
        let is_free = &self.0;
        if !is_free.load(Ordering::Relaxed) {
            return Local::now();
        }
        is_free.store(false, Ordering::Relaxed);
        let is_free_clone = is_free.clone();
        tokio::spawn(async move {
            <dyn ScheduleStrategy>::execute_logic(emitter, task).await;
            is_free_clone.store(true, Ordering::Relaxed);
        });
        Local::now()
    }
}