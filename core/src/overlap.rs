use std::sync::Arc;
use crate::task::{Task};

#[async_trait::async_trait]
pub trait OverlapStrategy: Send + Sync {
    async fn handle(&self, task: Arc<Task>);
}

pub struct SequentialOverlapPolicy;
#[async_trait::async_trait]
impl OverlapStrategy for SequentialOverlapPolicy {
    async fn handle(&self, task: Arc<Task>) {
        task.frame.execute(&*task.metadata).await.unwrap();
    }
}

pub struct ParallelOverlapPolicy;
#[async_trait::async_trait]
impl OverlapStrategy for ParallelOverlapPolicy {
    async fn handle(&self, task: Arc<Task>) {
        tokio::spawn(async move {
            task.frame.execute(&*task.metadata).await.unwrap();
        });
    }
}