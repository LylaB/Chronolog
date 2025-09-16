pub mod task_store;
pub mod task_dispatcher;

use crate::clock::SystemClock;
use std::sync::Arc;
use arc_swap::{ArcSwapOption};
use once_cell::sync::Lazy;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;
use typed_builder::TypedBuilder;
use crate::clock::SchedulerClock;
use crate::scheduler::task_dispatcher::{SchedulerTaskDispatcher, DefaultTaskDispatcher};
use crate::scheduler::task_store::{SchedulerTaskStore, EphemeralDefaultTaskStore};
use crate::task::{Task, TaskEventEmitter};

pub static CHRONOLOG_SCHEDULER: Lazy<Arc<Scheduler>> = Lazy::new(|| {
        Arc::new(Scheduler::builder().build())
});

#[derive(TypedBuilder)]
#[builder(build_method(into = Scheduler))]
pub struct SchedulerConfig {
    #[builder(
        default = DefaultTaskDispatcher::default_configs(),
        setter(transform = |std: impl SchedulerTaskDispatcher + 'static| Arc::new(std) as Arc<dyn SchedulerTaskDispatcher>),
    )]
    dispatcher: Arc<dyn SchedulerTaskDispatcher>,

    #[builder(
        default = EphemeralDefaultTaskStore::new(),
        setter(transform = |std: impl SchedulerTaskStore + 'static| Arc::new(std) as Arc<dyn SchedulerTaskStore>),
    )]
    store: Arc<dyn SchedulerTaskStore>,

    #[builder(
        default = Arc::new(SystemClock),
        setter(transform = |clock: impl SchedulerClock + 'static| Arc::new(clock) as Arc<dyn SchedulerClock>),
    )]
    clock: Arc<dyn SchedulerClock>,
}

impl From<SchedulerConfig> for Scheduler {
    fn from(config: SchedulerConfig) -> Self {
        let (schedule_tx, schedule_rx) = broadcast::channel(16);

        Self {
            dispatcher: config.dispatcher,
            store: config.store,
            clock: config.clock,
            process: ArcSwapOption::new(None),
            schedule_tx: Arc::new(schedule_tx),
            schedule_rx: Arc::new(Mutex::new(schedule_rx)),
            notifier: Arc::new(tokio::sync::Notify::new())
        }
    }
}

pub struct Scheduler {
    dispatcher: Arc<dyn SchedulerTaskDispatcher>,
    store: Arc<dyn SchedulerTaskStore>,
    clock: Arc<dyn SchedulerClock>,
    process: ArcSwapOption<JoinHandle<()>>,
    schedule_tx: Arc<broadcast::Sender<(Arc<Task>, usize)>>,
    schedule_rx: Arc<Mutex<broadcast::Receiver<(Arc<Task>, usize)>>>,
    notifier: Arc<tokio::sync::Notify>
}

impl Scheduler {
    pub fn builder() -> SchedulerConfigBuilder {
        SchedulerConfig::builder()
    }

    pub async fn start(&self) {
        let emitter = Arc::new(TaskEventEmitter {_private: ()});
        let store_clone = self.store.clone();
        let clock_clone = self.clock.clone();
        let dispatcher_clone = self.dispatcher.clone();
        let scheduler_send = self.schedule_tx.clone();
        let scheduler_receive = self.schedule_rx.clone();
        let notifier = self.notifier.clone();
        self.process.store(Some(Arc::new(
            tokio::spawn(async move {
                let double_clock_clone = clock_clone.clone();
                let double_store_clone = store_clone.clone();
                let double_notifier_clone = notifier.clone();
                tokio::spawn(async move {
                    while let Some((task, idx)) =
                        scheduler_receive.lock().await.recv().await.ok()
                    {
                        double_store_clone.reschedule(double_clock_clone.clone(), task, idx).await;
                        double_notifier_clone.notify_waiters();
                    }
                });

                loop {
                    if let Some((task, time, idx)) = store_clone.retrieve().await {
                        tokio::select! {
                            _ = clock_clone.idle_to(time) => {
                                store_clone.pop().await;
                                if !store_clone.exists(idx).await { continue; }
                                dispatcher_clone.clone()
                                .dispatch(scheduler_send.clone(), emitter.clone(), task, idx)
                                .await;
                                continue;
                            }

                            _ = notifier.notified() => {
                                continue;
                            }
                        }
                    }
                }
            })
        )))
    }

    pub async fn abort(&self) {
        let process = self.process.swap(None);
        process.map(|p| p.abort());
    }

    pub async fn schedule(&self, task: Task) -> usize {
        let idx = self.store.store(self.clock.clone(), task).await;
        idx
    }

    pub async fn cancel(&self, idx: usize) {
        self.store.remove(idx).await;
    }

    pub async fn exists(&self, idx: usize) {
        self.store.exists(idx).await;
    }

    pub async fn has_started(&self) -> bool {
        self.process.load().is_some()
    }
}