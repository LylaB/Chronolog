use std::num::NonZeroU64;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use arc_swap::ArcSwap;
use chrono::{DateTime, Local};
use uuid::Uuid;
use crate::task::Task;

/// The exposed immutable subset of [`TaskMetadata`], contains methods to get the metadata
/// of the task and a convenience method for getting the remaining runs via calling
/// [`ExposedTaskMetadata::remaining_runs`], read more the documentation of [`TaskMetadata`]
/// for what type of fields are available to access
///
/// # See
/// - [`TaskMetadata`]
pub trait ExposedTaskMetadata: Send + Sync {
    fn max_runs(&self) -> Option<NonZeroU64>;
    fn runs(&self) -> u64;
    fn last_execution(&self) -> Arc<DateTime<Local>>;
    fn debug_label(&self) -> &str;
    fn remaining_runs(&self) -> Option<NonZeroU64> {
        match self.max_runs() {
            Some(max_runs) => {
                Some(NonZeroU64::new(max_runs.get().saturating_sub(self.runs())).unwrap())
            }
            None => None,
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
///
/// By default, task metadata contains:
/// - **Maximum runs**, the maximum runs this task can run before stopping the rescheduling (by default
/// it is set to run for infinite times), accessed via [`TaskMetadata::max_runs`] or [`ExposedTaskMetadata::max_runs`]
///
/// - **Run Count**, the number of times this task has run throughout its lifetime, accessed via
/// [`TaskMetadata::runs`] or [`ExposedTaskMetadata::runs`]
///
/// - **Last Execution**, the point in which the task was last executed at, if the task hasn't
/// executed then it returns none, accessed via [`TaskMetadata::last_execution`] or
/// [`ExposedTaskMetadata::last_execution`]
///
/// - **Debug Label** a label (can be an ID, a name... etc.) for identifying a task, by default, it
/// constructs a UUID per task, can be accessed via [`TaskMetadata::max_runs`] or
/// [`ExposedTaskMetadata::max_runs`]
pub trait TaskMetadata: Send + Sync {
    /// Gets a mutable container (`AtomicU64`) of the maximum number of runs allowed
    fn max_runs(&self) -> Option<NonZeroU64>;

    /// Gets a mutable container (`AtomicU64`) of the number of times the task has run
    fn runs(&self) -> &AtomicU64;

    /// Gets a mutable container (`ArcSwap`) of the last execution date
    fn last_execution(&self) -> &ArcSwap<DateTime<Local>>;

    /// Reads the debug label of the task
    fn debug_label(&self) -> &str;

    /// Returns an exposed set of the task metadata
    fn as_exposed(&self) -> Arc<dyn ExposedTaskMetadata + Send + Sync>;
}

/// A default implementation of the [`TaskMetadata`], it contains the bare minimum information
/// to describe a [`TaskMetadata`], this is used for internally tracking state of the task.
/// Unless one wishes to track more state in task metadata, this container is the go-to default
/// option (which is why [`Task`] doesn't require the supply of metadata)
///
/// # See
/// - [`Task`]
/// - [`TaskMetadata`]
/// - [`DefaultTaskMetadataExposed`]
pub struct DefaultTaskMetadata {
    pub max_runs: Option<NonZeroU64>,
    pub runs: AtomicU64,
    pub last_execution: ArcSwap<DateTime<Local>>,
    pub debug_label: String,
}

impl DefaultTaskMetadata {
    pub fn new() -> Self {
        Self::new_with(Uuid::new_v4().to_string())
    }

    pub fn new_with(debug_label: String) -> Self {
        DefaultTaskMetadata {
            max_runs: None,
            runs: AtomicU64::new(0),
            last_execution: ArcSwap::from_pointee(Local::now()),
            debug_label
        }
    }
}

/// An exposed subset of the [`DefaultTaskMetadata`], containing the bare minimum information to
/// describe a [`TaskMetadata`], this subset is used for accessing fields. Unless one wishes to provide
/// more methods and fields for exposed task metadata, this is the go-to default option
///
/// # See
/// - [`Task`]
/// - [`TaskMetadata`]
/// - [`DefaultTaskMetadataExposed`]
pub struct DefaultTaskMetadataExposed {
    pub max_runs: Option<NonZeroU64>,
    pub runs: u64,
    pub last_execution: Arc<DateTime<Local>>,
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
}

impl TaskMetadata for DefaultTaskMetadata {
    fn max_runs(&self) -> Option<NonZeroU64> {
        self.max_runs
    }

    fn runs(&self) -> &AtomicU64 {
        &self.runs
    }

    fn last_execution(&self) -> &ArcSwap<DateTime<Local>> {
        &self.last_execution
    }

    fn debug_label(&self) -> &str {
        &self.debug_label
    }

    fn as_exposed(&self) -> Arc<dyn ExposedTaskMetadata + Send + Sync> {
        let loaded = self.last_execution.load().clone();
        Arc::new(DefaultTaskMetadataExposed {
            max_runs: self.max_runs,
            runs: self.runs.load(Ordering::Relaxed),
            last_execution: loaded.clone(),
            debug_label: self.debug_label.clone(),
        })
    }
}