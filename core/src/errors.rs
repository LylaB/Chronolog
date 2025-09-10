use crate::task::TaskError;
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChronologErrors {
    #[error("`{0}` Failed to successfully execute, the function returned an error: {1:?}")]
    FailedExecution(String, TaskError),

    #[error("`{0}` was aborted")]
    TaskAborted(String),

    #[error("`{0}` Timed out")]
    TimeoutError(String),
}
