use crate::scheduler::task_dispatcher::SchedulerTaskDispatcher;
use crate::task::{Task, TaskEventEmitter, TaskPriority};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::broadcast;
use typed_builder::TypedBuilder;

pub struct DispatcherThreadCount {
    pub critical: usize,
    pub important: usize,
    pub high: usize,
    pub moderate: usize,
    pub low: usize,
}

impl Default for DispatcherThreadCount {
    fn default() -> Self {
        Self {
            critical: 2,
            important: 4,
            high: 8,
            moderate: 16,
            low: 32,
        }
    }
}

#[derive(TypedBuilder)]
#[builder(build_method(into = Arc<DefaultTaskDispatcher>))]
pub struct DefaultTaskDispatcherConfig {
    workload: DispatcherThreadCount,
}

impl From<DefaultTaskDispatcherConfig> for Arc<DefaultTaskDispatcher> {
    fn from(value: DefaultTaskDispatcherConfig) -> Arc<DefaultTaskDispatcher> {
        Arc::new(DefaultTaskDispatcher {
            critical_pool: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(value.workload.critical)
                    .thread_name("critical-task-worker")
                    .build()
                    .unwrap(),
            ),
            important_pool: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(value.workload.important)
                    .thread_name("important-task-worker")
                    .build()
                    .unwrap(),
            ),
            high_pool: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(value.workload.high)
                    .thread_name("high-task-worker")
                    .build()
                    .unwrap(),
            ),
            moderate_pool: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(value.workload.moderate)
                    .thread_name("moderate-task-worker")
                    .build()
                    .unwrap(),
            ),
            low_pool: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(value.workload.low)
                    .thread_name("low-task-worker")
                    .build()
                    .unwrap(),
            ),
        })
    }
}

pub struct DefaultTaskDispatcher {
    critical_pool: Arc<Runtime>,
    important_pool: Arc<Runtime>,
    high_pool: Arc<Runtime>,
    moderate_pool: Arc<Runtime>,
    low_pool: Arc<Runtime>,
}

impl DefaultTaskDispatcher {
    pub fn default_configs() -> Arc<Self> {
        DefaultTaskDispatcher::builder()
            .workload(DispatcherThreadCount::default())
            .build()
    }

    pub fn builder() -> DefaultTaskDispatcherConfigBuilder {
        DefaultTaskDispatcherConfig::builder()
    }
}

#[async_trait]
impl SchedulerTaskDispatcher for DefaultTaskDispatcher {
    async fn dispatch(
        self: Arc<Self>,
        sender: Arc<broadcast::Sender<(Arc<Task>, usize)>>,
        emitter: Arc<TaskEventEmitter>,
        task: Arc<Task>,
        idx: usize,
    ) {
        let target_pool = match task.priority {
            TaskPriority::CRITICAL => self.critical_pool.clone(),

            TaskPriority::IMPORTANT => self.important_pool.clone(),

            TaskPriority::HIGH => self.high_pool.clone(),

            TaskPriority::MODERATE => self.moderate_pool.clone(),

            TaskPriority::LOW => self.low_pool.clone(),
        };

        target_pool.spawn(async move {
            task.clone()
                .overlap_policy
                .handle(task.clone(), emitter)
                .await;
            sender.send((task, idx)).unwrap();
        });
    }
}
