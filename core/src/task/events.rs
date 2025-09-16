use crate::task::metadata::ExposedTaskMetadata;
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type TaskStartEvent = ArcTaskEvent<()>;
pub type TaskEndEvent = ArcTaskEvent<Option<TaskError>>;
pub type ArcTaskEvent<P> = Arc<TaskEvent<P>>;
pub type TaskError = Arc<dyn Debug + Send + Sync>;

pub type DynListenerFunc<P> = Arc<dyn Fn(Arc<dyn ExposedTaskMetadata>, P) + Send + Sync>;

/// [`TaskEvent`] defines an event which may (or may not, depending on how the frame implementation
/// handles this task event) execute. This is the main system used for listening to various events,
/// there are 2 types of events at play, which one can listen to:
///
/// - **Lifecycle Task Events** These are automatically emitted by the scheduler, all task frames have this
///   event no matter the type. Currently, there are 2 of these, the first namely being ``on_start``
///   used for listening to when a task is about to start. While the second is ``on_end`` which is used
///   for listening to when a task is ending (this event executes before the error handler)
///
/// - **Local Task Events** These are local to the task frame, different task frames may have none, one
///   or multiple of these event types. They are emitted by the task frame logic and give more extensibility
///   to what outside parties can listen to (for example, on the fallback task frame, one can listen to
///   when the fallback is about to execute)
///
/// [`TaskEvent`] **CANNOT** be emitted by itself, it needs an emitter which is only handed to the
/// scheduler, overlapping policies and the task frame. Outside parties can listen to the event at any
/// time they would like
pub struct TaskEvent<P> {
    listeners: DashMap<Uuid, DynListenerFunc<P>>,
}

impl<P: Send + 'static> TaskEvent<P> {
    /// Creates a task event, containing no listeners
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            listeners: DashMap::new(),
        })
    }

    /// Subscribes a listener to the task event, returning an identifier for that listener / subscriber
    pub async fn subscribe(
        &self,
        func: impl Fn(Arc<dyn ExposedTaskMetadata>, P) + Send + Sync + 'static,
    ) -> Uuid {
        let id = Uuid::new_v4();
        self.listeners.insert(id, Arc::new(func));
        id
    }

    /// Unsubscribes a listener to the task event, returning an identifier for that listener
    pub async fn unsubscribe(&self, id: Uuid) {
        self.listeners.remove(&id);
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
    pub async fn emit<P: Send + Sync + Clone>(
        &self,
        metadata: Arc<dyn ExposedTaskMetadata>,
        event: Arc<TaskEvent<P>>,
        payload: P,
    ) {
        std::thread::scope(|s| {
            for listener in event.listeners.iter() {
                let cloned_listener = listener.value().clone();
                let cloned_metadata = metadata.clone();
                let cloned_payload = payload.clone();
                s.spawn(move || {
                    cloned_listener(cloned_metadata, cloned_payload);
                });
            }
        });
    }
}
