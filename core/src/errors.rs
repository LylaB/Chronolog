use std::fmt::{Debug};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChronologErrors {
    #[error("`{0}` Failed to successfully execute, the function returned an error: {1:?}")]
    FailedExecution(String, Box<dyn Debug + Send + Sync>),

    #[error("`{0}` Timed out")]
    TimeoutError(String)
}