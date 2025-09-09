pub mod execution;
pub mod sequential;
pub mod retry;
pub mod timeout;
pub mod fallback;
pub mod parallel;

use std::any::Any;
pub use execution::ExecutionTaskFrame;
pub use sequential::SequentialTask;
pub use parallel::ParallelTask;
pub use timeout::TimeoutTask;
pub use fallback::FallbackTask;
pub use retry::RetriableTask;

use std::error::Error;
use std::fmt::Debug;
use std::num::NonZeroU64;
use std::ops::Deref;
use std::sync::{Arc};
use std::sync::atomic::{AtomicU64, Ordering};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use typed_builder::TypedBuilder;
use crate::overlap::{OverlapStrategy, SequentialOverlapPolicy};
use crate::schedule::Schedule;

#[derive(TypedBuilder)]
pub struct Task {
    #[builder(default = Arc::new(DefaultTaskMetadata::new()))]
    pub metadata: Arc<dyn TaskMetadata>,

    #[builder(setter(transform = |s: impl TaskFrame + 'static| Arc::new(s) as Arc<dyn TaskFrame>))]
    pub frame: Arc<dyn TaskFrame>,

    #[builder(setter(transform = |s: impl Schedule + 'static| Arc::new(s) as Arc<dyn Schedule>))]
    pub schedule: Arc<dyn Schedule>,
}

/*
/// [`LocalTaskEvent`] defines an event which may (or may not, depending on how the frame implementation
/// handles this task event) execute, this is one of the two systems for events. It allows for fine-grain
/// control depending on the [`TaskFrame`] being listened to (task frames fully control what events it
/// defines). While one can manually implement events and handle it on their own, it is advised to
/// use this trait over manual implementation for consistency
#[async_trait]
pub trait LocalTaskEvent: Send + Sync {
    type Payload;
    type Frame: TaskFrame<_>;

    async fn emit(&self, payload: Self::Payload) -> &Self;
    async fn listen(&self, func: impl Fn(&Self::Frame, &Self::Payload) + Send + Sync + 'static);
}
 */

/// The exposed immutable subset of [`TaskMetadata`], contains methods to get the metadata
/// of the task and a convenience method for getting the remaining runs via calling
/// [`ExposedTaskMetadata::remaining_runs`]
///
/// # See
/// - [`TaskMetadata`]
pub trait ExposedTaskMetadata {
    fn max_runs(&self) -> Option<NonZeroU64>;
    fn runs(&self) -> u64;
    fn last_execution(&self) -> Arc<DateTime<Local>>;
    fn debug_label(&self) -> &str;
    fn overlap_policy(&self) -> Arc<dyn OverlapStrategy>;
    fn remaining_runs(&self) -> Option<NonZeroU64> {
        match self.max_runs() {
            Some(max_runs) => Some(
                NonZeroU64::new(max_runs.get().saturating_sub(self.runs())).unwrap()
            ),
            None => None
        }
    }
}

/// [`TaskMetadata`] is a container hosting all metadata-related information in which one can
/// modify internally and expose an immutable metadata subset. It contains common internal metadata
/// such as run count and last execution time (handled by the scheduling logic) as well
/// as non-internal metadata, i.e. Debug label and maximum run count (set by the user)
///
/// [`TaskMetadata`] can be exposed as a reference. Providing an immutable subset slice of the main
/// container (hosts the same metadata as the main container, however, the key difference is it does
/// not allow any mutability and can have fewer fields than the container)
pub trait TaskMetadata: Send + Sync {
    /// Gets a mutable container (`AtomicU64`) of the maximum number of runs allowed
    fn max_runs(&self) -> Option<NonZeroU64>;

    /// Gets a mutable container (`AtomicU64`) of the number of times the task has run
    fn run_count(&self) -> &AtomicU64;

    /// Gets a mutable container (`ArcSwap`) of the last execution date
    fn last_exec(&self) -> &ArcSwap<DateTime<Local>>;

    /// Reads the debug label of the task
    fn debug_label(&self) -> &str;

    /// Reads the overlapping policy / strategy used by the task
    fn overlap_policy(&self) -> Arc<dyn OverlapStrategy>;

    /// Returns an exposed set of the task metadata
    fn as_exposed(&self) -> Arc<dyn ExposedTaskMetadata>;
}

pub struct DefaultTaskMetadata {
    pub max_runs: Option<NonZeroU64>,
    pub runs: AtomicU64,
    pub last_execution: ArcSwap<DateTime<Local>>,
    pub overlap_policy: Arc<dyn OverlapStrategy>,
    pub debug_label: String,
}

impl DefaultTaskMetadata {
    pub fn new() -> Self {
        DefaultTaskMetadata {
            max_runs: None,
            runs: AtomicU64::new(0),
            last_execution: ArcSwap::from_pointee(Local::now()),
            overlap_policy: Arc::new(SequentialOverlapPolicy),
            debug_label: uuid::Uuid::new_v4().to_string(),
        }
    }
}

pub struct DefaultTaskMetadataExposed {
    pub max_runs: Option<NonZeroU64>,
    pub runs: u64,
    pub last_execution: Arc<DateTime<Local>>,
    pub overlap_policy: Arc<dyn OverlapStrategy>,
    pub debug_label: String,
}

impl ExposedTaskMetadata for DefaultTaskMetadataExposed {
    fn max_runs(&self) -> Option<NonZeroU64> {
        self.max_runs
    }

    fn runs(&self) -> u64 {
        self.runs
    }

    fn last_execution(&self) -> Arc<DateTime<Local>> {
        self.last_execution.clone()
    }

    fn debug_label(&self) -> &str {
        &self.debug_label
    }

    fn overlap_policy(&self) -> Arc<dyn OverlapStrategy> {
        self.overlap_policy.clone()
    }
}

impl TaskMetadata for DefaultTaskMetadata {
    fn max_runs(&self) -> Option<NonZeroU64> {
        self.max_runs
    }

    fn run_count(&self) -> &AtomicU64 {
        &self.runs
    }

    fn last_exec(&self) -> &ArcSwap<DateTime<Local>> {
        &self.last_execution
    }

    fn debug_label(&self) -> &str {
        &self.debug_label
    }

    fn overlap_policy(&self) -> Arc<dyn OverlapStrategy> {
        self.overlap_policy.clone()
    }

    fn as_exposed(&self) -> Arc<dyn ExposedTaskMetadata> {
        let loaded = self.last_execution.load().clone();
        Arc::new(DefaultTaskMetadataExposed {
            max_runs: self.max_runs.clone(),
            runs: self.runs.load(Ordering::Relaxed),
            last_execution: loaded.clone(),
            overlap_policy: self.overlap_policy.clone(),
            debug_label: self.debug_label.clone()
        })
    }
}

/// [`TaskFrame`] represents a unit of work which hosts the actual computation logic for the [`Scheduler`]
/// to invoke, this is a part of the task system. [`TaskFrame`] encapsulates mainly the async execution logic
/// of the task which is evaluated by calling [`TaskFrame::execute`] Returning `Ok(())` on success,
/// or an `Arc<dyn Error>` on failure, as well as events
///
/// # Notes
/// - This is one of many components which are combined to form a task, other components are needed
/// to fuse them to a task.
///
/// - [`TaskFrame`] can be decorated with other task unit implementations to expand the behavior, such
///     as adding retry mechanism via [`RetryFrame`], adding timeout via [`TimeoutFrame`]... etc.
///
///     ## Examples Of Frame Decorators
///     - **``RetryFrame<TimeoutFrame<T>>``**: Execute task `T`, when the task succeeds within a maximum
///     duration of `D` (can be controlled by the developer) then finish, otherwise
///     if it exceeds its maximum duration or if the task failed then abort it and retry it again,
///     repeating this process `N` times (can be controlled by the developer) with a delay per task
///     (can be controlled by the developer) `d`
///
///     - **``FallbackFrame<Timeout<T1>, RetryFrame<T2>>``**: Execute task `T1`, when the task succeeds within
///     a maximum duration of `D` (can be controlled by the developer) then finish, otherwise if it
///     either fails or it reaches its maximum duration then execute task `T2` (as a fallback), try/retry
///     executing this task for `N` times (can be controlled by the developer) with a delay per retry of
///     `d` (can be controlled by the developer), regardless if it succeeds at some time or fails entirely,
///     return the result back
#[async_trait]
pub trait TaskFrame: Send + Sync {
    /// The main execution logic of the task, it is meant as an internal method
    async fn execute(&self, metadata: &(dyn TaskMetadata + Send + Sync)) -> Result<(), Arc<dyn Debug + Send + Sync>>;

}