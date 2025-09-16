use std::sync::Arc;
use async_trait::async_trait;
use crate::task::{Task, TaskError};
use crate::task::metadata::ExposedTaskMetadata;

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
