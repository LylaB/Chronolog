pub mod execution;
pub mod sequential;
pub mod retry;
pub mod timeout;
pub mod fallback;
pub mod parallel;

pub use execution::ExecutionTask;
pub use sequential::SequentialTask;
pub use parallel::ParallelTask;
pub use timeout::TimeoutTask;
pub use fallback::FallbackTask;
pub use retry::RetriableTask;

use std::error::Error;
use std::num::{NonZeroU64};
use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use tokio_util::sync::CancellationToken;
use crate::errors::ChronologErrors;
use crate::overlap::OverlapStrategy;
use crate::schedule::Schedule;

/// [`Task`] represents a unit of work that can be scheduled and executed by a [`Scheduler`].
///
/// Each task encapsulates the following:
/// - its execution logic via `execute_inner(&self)`,
/// - metadata such as run counts and last execution time,
/// - scheduling information via `Schedule`,
/// - and concurrency/overlap behavior via `OverlapStrategy`.
///
/// # Execution
///
/// Tasks are executed via the [`process`] method, which combines the task’s `execute_inner`
/// logic with a [`CancellationToken`] to allow preemptive cancellation. This ensures tasks
/// can safely run in parallel or be aborted by the scheduler or overlap policies.
///
/// # Methods
///
/// - [`Task::execute_inner`] The core async logic of the task. Returns `Ok(())` on success,
/// or an `Arc<dyn Error>` on failure. This method acts as an internal one
/// - [`Task::process`] Executes the task while observing the provided `CancellationToken`. Returns an error
///  if execution was aborted. This method is the main entrypoint for execution and **SHOULD NOT** be overridden
/// - [`Task::get_schedule`] Returns the schedule that controls when this task should be executed.
/// - [`Task::total_runs`] / [`Task::set_total_runs`] Get or update the total number of times the task has executed.
/// - [`Task::maximum_runs`] / [`Task::set_maximum_runs`]  Get or set the maximum number of times
/// the task is allowed to run. Returns `None` if the maximum tasks allowed are unlimited
/// - [`Task::remaining_runs`] Calculates how many runs remain before reaching `maximum_runs`. Returns `None` if
/// unlimited. This method is mostly a helper one and **SHOULD NOT** be overridden
/// - [`Task::set_last_execution`] / [`Task::last_execution`] Get or update the timestamp of the last execution.
/// - [`Task::get_debug_label`] Returns a human-readable label useful for logging or debugging.
/// - [`Task::overlap_policy`] Returns the [`OverlapStrategy`] that defines how this task behaves when
/// overlapping executions occur (e.g., sequential, parallel, replace previous/current).
///
/// # Notes
/// - All methods are async to allow concurrent-safe access to internal state and metadata.
///
/// - The scheduler is responsible for mutating metadata (runs, last execution) and for
///   triggering task execution according to the schedule.
///
/// - Tasks should not modify their own scheduling or run-count metadata outside the
///   scheduler context.
#[async_trait]
pub trait Task: Send + Sync {
    /// The main execution logic of the task, it is meant as an internal method
    async fn execute_inner(&self) -> Result<(), Arc<dyn Error + Send + Sync>>;

    /// The process entrypoint, which is where the scheduler will call the task, this method
    /// **SHOULD NOT** be overridden. Instead, override [`Task::execute_inner`] to facilitate any
    /// execution logic
    async fn process(&self, cancellation_token: Arc<CancellationToken>) -> Result<(), Arc<dyn Error + Send + Sync>> {
        tokio::select! {
            res = self.execute_inner() => res,
            _ = cancellation_token.cancelled() => {
                Err(Arc::new(ChronologErrors::FailedExecution(
                    self.get_debug_label().await,
                    Box::new("Task was aborted".to_owned()),
                )))
            }
        }
    }

    /// Gets the schedule
    async fn get_schedule(&self) -> Arc<dyn Schedule>;

    /// Gets the total number of runs
    async fn total_runs(&self) -> u64;

    /// Gets the maximum runs this task allows, it may return `None` if they are unlimited
    async fn maximum_runs(&self) -> Option<NonZeroU64>;

    /// Sets the maximum runs allowed by the task
    async fn set_maximum_runs(&mut self, max_runs: NonZeroU64);

    /// Calculates and returns the remaining number of runs, it may return `None` if they are unlimited.
    /// This acts as a helper method and **SHOULD NOT** be overridden
    async fn remaining_runs(&self) -> Option<u64> {
        if let Some(max_runs) = self.maximum_runs().await {
            return Some(self.total_runs().await - max_runs.get());
        }
        None
    }

    /// Sets the total runs, this is an internal method used by the scheduler. It **SHOULD NOT** be
    /// used outside the scheduler
    async fn set_total_runs(&mut self, runs: u64);

    /// Sets the last time the task was executed, this is an internal method used by the scheduler.
    /// It **SHOULD NOT** be used outside the scheduler
    async fn set_last_execution(&mut self, exec: DateTime<Local>);

    /// Gets a human-readable label, useful for logging/debugging
    async fn get_debug_label(&self) -> String;

    /// Gets the last time the task was executed
    async fn last_execution(&self) -> DateTime<Local>;

    /// Gets the overlapping policy / strategy used by the task
    async fn overlap_policy(&self) -> Arc<dyn OverlapStrategy>;
}