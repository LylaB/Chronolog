pub mod executionframe;
pub mod fallbackframe;
pub mod parallelframe;
pub mod retryframe;
pub mod sequentialframe;
pub mod timeoutframe;
pub mod selectframe;
pub mod conditionframe;

pub use executionframe::ExecutionTaskFrame;
pub use fallbackframe::FallbackTaskFrame;
pub use parallelframe::ParallelTaskFrame;
pub use retryframe::RetriableTaskFrame;
pub use sequentialframe::SequentialTaskFrame;
pub use timeoutframe::TimeoutTaskFrame;
pub use selectframe::SelectTaskFrame;
pub use conditionframe::ConditionalFrame;

use crate::schedule::TaskSchedule;
use crate::scheduling_strats::{ScheduleStrategy, SequentialSchedulingPolicy};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::num::NonZeroU64;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use typed_builder::TypedBuilder;
use uuid::Uuid;

pub type TaskStartEvent = ArcTaskEvent<()>;
pub type TaskEndEvent = ArcTaskEvent<Option<TaskError>>;
pub type ArcTaskEvent<P> = Arc<TaskEvent<P>>;
pub type TaskError = Arc<dyn Debug + Send + Sync>;

/// [`Task`] is one of the core components of Chronolog, it is a composite, and made of several parts,
/// giving it massive flexibility in terms of customization.
///
/// # Task Composite Parts
///
/// - **[`TaskMetadata`]** The <u>State</u>, by default (the parameter is optional to define)
/// it contains information such as the run-count, the maximum runs allowed, the last time the task
/// was executed... etc. The task metadata can be exposed in the form of [`ExposedTaskMetadata`],
/// giving an immutable version of it, typically this metadata is exposed to the task frame, the error
/// handler and this exposed version can be used outside via [`Task::metadata`]
///
/// - **[`TaskFrame`]** The <u>What</u> of the task, the logic part of the task. When executed, task
/// frames get the exposed metadata and an event emitter for task events (lifecycle or local events,
/// see [`TaskEvent`] for more context), the emitter can be used to emit their own events. Task frames
/// can be decorated with other task frames to form a chain of task frames, allowing for complex
/// logic (and policy logic) to be injected to the task without manual writing. There are various
/// implementations of task frane and the task frame can be accessed via [`Task::frame`]
///
/// - **[`TaskSchedule`]** The <u>When</u> will the task execute, it is used for calculating the next
/// time to invoke this task. This part is useful to the scheduler mostly, tho outside parties can
/// also use it via [`Task::schedule`]
///
/// - **[`TaskErrorHandler`]** An error handler for the task, in case things go south. By default,
/// it doesn't need to be supplied, and it will silently ignore the error, tho ideally in most cases
/// it should be supplied for fine-grain error handling. When invoked, the task error handler gets
/// a context object hosting the exposed metadata and the error. It is meant to return nothing, just
/// handle the error the task gave away
///
/// - **[`ScheduleStrategy`]** Defines how it is scheduled and how it handles task overlapping
/// behavior. By default, (the parameter is optional to define), it runs sequentially. i.e. The task
/// only reschedules once it is fully finished
/// ---
///
/// In order to actually use the task, the developer must register it in a [`Scheduler`], could be
/// the default implementation of the scheduler or a custom-made, regardless, the task object is useless
/// without registration of it
///
/// # See
/// - [`TaskFrame`]
/// - [`TaskMetadata`]
/// - [`ExposedTaskMetadata`]
/// - [`Scheduler`]
/// - [`TaskEvent`]
/// - [`TaskSchedule`]
/// - [`ScheduleStrategy`]
/// - [`TaskErrorHandler`]
#[derive(TypedBuilder)]
pub struct Task {
    #[builder(default = Arc::new(DefaultTaskMetadata::new()))]
    pub(crate) metadata: Arc<dyn TaskMetadata>,

    #[builder(setter(transform = |s: impl TaskFrame + 'static| Arc::new(s) as Arc<dyn TaskFrame>))]
    pub(crate) frame: Arc<dyn TaskFrame>,

    #[builder(setter(transform = |s: impl TaskSchedule + 'static| Arc::new(s) as Arc<dyn TaskSchedule>))]
    pub(crate) schedule: Arc<dyn TaskSchedule>,

    #[builder(
        default = Arc::new(SilentTaskErrorHandler),
        setter(transform = |s: impl TaskErrorHandler + 'static| Arc::new(s) as Arc<dyn TaskErrorHandler>)
    )]
    pub(crate) error_handler: Arc<dyn TaskErrorHandler>,

    #[builder(
        default = Arc::new(SequentialSchedulingPolicy),
        setter(transform = |s: impl ScheduleStrategy + 'static| Arc::new(s) as Arc<dyn ScheduleStrategy>)
    )]
    pub(crate) overlap_policy: Arc<dyn ScheduleStrategy>,
}

impl Task {
    /// Creates a simple task from a schedule and an interval. Mostly used as a convenient method
    /// for simple enough tasks that don't need any of the other composite parts
    pub fn define(schedule: impl TaskSchedule + 'static, task: impl TaskFrame + 'static) -> Self {
        Self {
            frame: Arc::new(task),
            metadata: Arc::new(DefaultTaskMetadata::new()),
            schedule: Arc::new(schedule),
            error_handler: Arc::new(SilentTaskErrorHandler),
            overlap_policy: Arc::new(SequentialSchedulingPolicy),
        }
    }

    /// Gets the exposed metadata (immutable) for outside parties
    pub fn metadata(&self) -> Arc<dyn ExposedTaskMetadata> {
        self.metadata.as_exposed().clone()
    }

    /// Gets the frame for outside parties
    pub fn frame(&self) -> Arc<dyn TaskFrame> {
        self.frame.clone()
    }

    /// Gets the schedule for outside parties
    pub fn schedule(&self) -> Arc<dyn TaskSchedule> {
        self.schedule.clone()
    }

    /// Gets the error handler for outside parties
    pub fn error_handler(&self) -> Arc<dyn TaskErrorHandler> {
        self.error_handler.clone()
    }

    /// Gets the overlapping policy for outside parties
    pub fn overlap_policy(&self) -> Arc<dyn ScheduleStrategy> {
        self.overlap_policy.clone()
    }
}

/// [`TaskEvent`] defines an event which may (or may not, depending on how the frame implementation
/// handles this task event) execute. This is the main system used for listening to various events,
/// there are 2 types of events at play, which one can listen to:
///
/// - **Lifecycle Task Events** These are automatically emitted by the scheduler, all task frames have this
/// event no matter the type. Currently, there are 2 of these, the first namely being ``on_start``
/// used for listening to when a task is about to start. While the second is ``on_end`` which is used
/// for listening to when a task is ending (this event executes before the error handler)
///
/// - **Local Task Events** These are local to the task frame, different task frames may have none, one
/// or multiple of these event types. They are emitted by the task frame logic and give more extensibility
/// to what outside parties can listen to (for example, on the fallback task frame, one can listen to
/// when the fallback is about to execute)
///
/// [`TaskEvent`] **CANNOT** be emitted by itself, it needs an emitter which is only handed to the
/// scheduler, overlapping policies and the task frame. Outside parties can listen to the event at any
/// time they would like
pub struct TaskEvent<P> {
    listeners: RwLock<HashMap<Uuid, Arc<dyn Fn(Arc<dyn ExposedTaskMetadata>, &P) + Send + Sync>>>,
}

impl<P: Send + 'static> TaskEvent<P> {
    /// Creates a task event, containing no listeners
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            listeners: RwLock::new(HashMap::new()),
        })
    }

    /// Subscribes a listener to the task event, returning an identifier for that listener / subscriber
    pub async fn subscribe(
        &self,
        func: impl Fn(Arc<dyn ExposedTaskMetadata>, &P) + Send + Sync + 'static,
    ) -> Uuid {
        let id = Uuid::new_v4();
        self.listeners.write().await.insert(id, Arc::new(func));
        id
    }

    /// Unsubscribes a listener to the task event, returning an identifier for that listener
    pub async fn unsubscribe(&self, id: Uuid) {
        self.listeners.write().await.remove(&id);
    }
}

/// [`TaskEventEmitter`] is a sealed mechanism to allow the use of emitting events, by itself
/// it doesn't hot any data, but it unlocks the use of [`TaskEventEmitter::emit`].
/// The reason for this is to prevent any emissions from outside parties on events
pub struct TaskEventEmitter {
    pub(crate) _private: (),
}

impl TaskEventEmitter {
    /// Emits the event, notifying all subscribers / listeners
    pub async fn emit<P: Send + Sync>(
        &self,
        metadata: Arc<dyn ExposedTaskMetadata>,
        event: Arc<TaskEvent<P>>,
        payload: P,
    ) {
        let listeners = &*event.listeners.read().await;
        match listeners.len() {
            0 => {}
            1 => listeners.values().next().unwrap()(metadata.clone(), &payload),
            _ => {
                std::thread::scope(|s| {
                    for listener in listeners.values() {
                        s.spawn(|| {
                            listener(metadata.clone(), &payload);
                        });
                    }
                });
            }
        };
    }
}

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
        DefaultTaskMetadata {
            max_runs: None,
            runs: AtomicU64::new(0),
            last_execution: ArcSwap::from_pointee(Local::now()),
            debug_label: Uuid::new_v4().to_string(),
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
            max_runs: self.max_runs.clone(),
            runs: self.runs.load(Ordering::Relaxed),
            last_execution: loaded.clone(),
            debug_label: self.debug_label.clone(),
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
///     - **``RetriableTaskFrame<TimeoutTaskFrame<T>>``**: Execute task frame `T`, when the
///     task frame succeeds within a maximum duration of `D` (can be controlled by the developer)
///     then finish, otherwise if it exceeds its maximum duration or if the task frame failed then
///     abort it and retry it again, repeating this process `N` times (can be controlled by the developer)
///     with a delay per retry (can be controlled by the developer) `d`
///
///     - **``FallbackTaskFrame<TimeoutTaskFrame<T1>, RetriableTaskFrame<T2>>``**: Execute task frame `T1`,
///     when the task frame succeeds within a maximum duration of `D` (can be controlled by the developer)
///     then finish, otherwise if it either fails or it reaches its maximum duration then execute
///     task frame `T2` (as a fallback), try/retry executing this task frame for `N` times (can be
///     controlled by the developer) with a delay per retry of `d` (can be controlled by the developer),
///     regardless if it succeeds at some time or fails entirely, return the result back
#[async_trait]
pub trait TaskFrame: Send + Sync {
    /// The main execution logic of the task, it is meant as an internal method
    async fn execute(
        &self,
        metadata: Arc<dyn ExposedTaskMetadata + Send + Sync>,
        emitter: Arc<TaskEventEmitter>,
    ) -> Result<(), TaskError>;

    fn on_start(&self) -> TaskStartEvent;
    fn on_end(&self) -> TaskEndEvent;
}

/// An error context object, it cannot be created by outside parties and is handed by the
/// scheduling strategy to control. The error context contains the error and an exposed set of
/// metadata in which the fields can be accessed via [`TaskErrorContext::error`]
/// and [`TaskErrorContext::metadata`] respectively
pub struct TaskErrorContext {
    pub(crate) error: TaskError,
    pub(crate) metadata: Arc<dyn ExposedTaskMetadata + Send + Sync>,
}

impl TaskErrorContext {
    /// Gets the task error
    pub fn error(&self) -> TaskError {
        self.error.clone()
    }

    /// Gets the exposed metadata
    pub fn metadata(&self) -> Arc<dyn ExposedTaskMetadata + Send + Sync> {
        self.metadata.clone()
    }
}

/// [`TaskErrorHandler`] is a logic part that deals with any errors, it is invoked when a task has
/// returned an error. It is executed after the `on_end` [`TaskEvent`], the handler returns nothing
/// back and its only meant to handle errors (example a rollback mechanism, assuming a default value
/// for some state... etc.). By default, the error handler supplied to the task is [`SilentTaskErrorHandler`]
///
/// # See
/// - [`TaskErrorHandler`]
/// - [`TaskEvent`]
/// - [`SilentTaskErrorHandler`]
#[async_trait]
pub trait TaskErrorHandler: Send + Sync {
    async fn on_error(&self, context: TaskErrorContext);
}

/// An implementation of [`TaskErrorHandler`] to panic, this should not be used in production-grade
/// applications, it is recommended to handle errors with your own logic
/// # See
/// - [`TaskErrorHandler`]
pub struct PanicTaskErrorHandler;

#[async_trait]
impl TaskErrorHandler for PanicTaskErrorHandler {
    async fn on_error(&self, context: TaskErrorContext) {
        panic!("{:?}", context.error);
    }
}

/// An implementation of [`TaskErrorHandler`] to silently ignore errors, in most cases this
/// should not be used in production-grade applications as it makes debugging harder, however,
/// for small demos, or if all the possible errors do not contain any valuable information
///
/// This is the default option for [`Task`]
///
/// # See
/// - [`Task`]
pub struct SilentTaskErrorHandler;

#[async_trait]
impl TaskErrorHandler for SilentTaskErrorHandler {
    async fn on_error(&self, _context: TaskErrorContext) {}
}
