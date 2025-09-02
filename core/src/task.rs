pub mod execution;
pub mod sequential;
pub mod retry;
pub mod timeout;
pub mod fallback;
pub mod parallel;

use std::error::Error;
use std::fmt::Debug;
use std::num::{NonZeroU64};
use std::sync::Arc;
use std::time::{Duration};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use tokio_util::sync::CancellationToken;
use crate::errors::ChronologErrors;
use crate::overlap::OverlapStrategy;
use crate::schedule::Schedule;

#[macro_export]
macro_rules! task_fn {
    ($metadata:ident => $body:block) => {
        |$metadata| Box::pin(async move $body)
    };
}

#[async_trait]
pub trait Task: Send + Sync {
    async fn execute_inner(&self) -> Result<(), Arc<dyn Error + Send + Sync>>;

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

    async fn get_schedule(&self) -> &Schedule;

    async fn total_runs(&self) -> u64;
    async fn maximum_runs(&self) -> Option<NonZeroU64>;
    async fn set_maximum_runs(&mut self, max_runs: NonZeroU64);

    async fn remaining_runs(&self) -> Option<u64> {
        if let Some(max_runs) = self.maximum_runs().await {
            return Some(self.total_runs().await - max_runs.get());
        }
        None
    }

    async fn set_total_runs(&mut self, runs: u64);

    async fn set_last_execution(&mut self, exec: DateTime<Local>);

    async fn get_debug_label(&self) -> String;

    async fn last_execution(&self) -> DateTime<Local>;

    async fn overlap_policy(&self) -> Arc<dyn OverlapStrategy>;
}