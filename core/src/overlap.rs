use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use crate::scheduler::TaskEntry;
use crate::task::Task;

#[derive(Clone)]
pub struct OverlapContext<'a>(pub(crate) &'a TaskEntry);

impl<'a> OverlapContext<'a> {
    pub fn get_task(&self) -> Arc<Mutex<dyn Task>> {
        self.0.task.clone()
    }

    pub fn set_process(&self, handle: JoinHandle<()>, token: Arc<CancellationToken>) {
        self.0.process.store(Some(Arc::new(handle)));
        self.0.cancel_token.store(Some(token));
    }

    pub fn has_running(&self) -> bool {
        self.0.process.load().is_some()
    }

    pub fn abort_running(&self) {
        if let (Some(handle), Some(token)) = (
            &*self.0.process.load(),
            &*self.0.cancel_token.load()
        ) {
            handle.abort();
            token.cancel();
        }
    }
}

#[async_trait::async_trait]
pub trait OverlapStrategy: Send + Sync {
    async fn execute(&self, task_entry: &OverlapContext<'_>);
}

pub struct RerunAfterCompletion;
#[async_trait::async_trait]
impl OverlapStrategy for RerunAfterCompletion {
    async fn execute(&self, ctx: &OverlapContext<'_>) {
        let task = ctx.get_task();
        let lock = task.lock().await;
        let token = Arc::new(CancellationToken::new());
        lock.process(token).await.unwrap();
    }
}