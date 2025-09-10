use tokio::sync::RwLock;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::Duration;
use arc_swap::ArcSwapOption;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use nohash_hasher::IntMap;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Instant};
use crate::task::{Task, TaskEventEmitter};

pub static CHRONOLOG_SCHEDULER: Lazy<Arc<ChronologScheduler>> = Lazy::new(|| ChronologScheduler::new());

pub(crate) struct ScheduledItem(pub(crate) usize, pub(crate) DateTime<Local>);

impl Eq for ScheduledItem {}

impl PartialEq<Self> for ScheduledItem {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl PartialOrd<Self> for ScheduledItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.1.partial_cmp(&other.1)
    }
}

impl Ord for ScheduledItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(&other.1)
    }
}

/// The [`Scheduler`] trait defines a basis for what a scheduler is, a scheduler is the actual mechanism
/// that drives the execution of multiple [`Task`] structs in specific times the tasks request,
/// typically this is mainly for extensibility. In most scenarios, the default global scheduler
/// which is [`CHRONOLOG_SCHEDULER`] or the struct [`ChronologScheduler`] can be used for the scheduling logic
///
/// All methods are async and in addition, require `&Arc<Self>` as opposed to typical `&self`,
/// for multi-thread and ergonomic reasons
///
/// The main internal logic side of the scheduler is [`Scheduler::main`], it cannot be called from any
/// outsiders as it requires an event emitter, created inside the [`Scheduler::start`] method,
/// by design, schedulers explicitly require the developer to call [`Scheduler::start`] to start it,
/// schedulers can also be aborted via the method [`Scheduler::abort`], do note that the
/// [`Scheduler::start`] **should not be overridden, due to the event emitter**
///
/// For registering and removing tasks, the following methods may be used
/// - [`Scheduler::register`] Registers a task to the scheduler, tasks can be inserted at any time
/// either when the scheduler hasn't started yet or even when the scheduler already has begun, the
/// method returns the index corresponding to the task (if later the developer wants to remove it)
///
/// - [`Scheduler::cancel`] Cancels the execution of a task (i.e. Removes the task from the scheduler),
///  in order to remove the task, one has to supply the index of that task
///
/// - [`Scheduler::clear`] Removes / clears all tasks the scheduler has registered
#[async_trait]
pub trait Scheduler: 'static + Send + Sync {
    /// Start the scheduler, if the scheduler is already started, then do nothing
    async fn start(self: &'static Arc<Self>) {
        let emitter = Arc::new(TaskEventEmitter { _private: () });
        self.main(emitter).await;
    }

    async fn main(self: &'static Arc<Self>, emitter: Arc<TaskEventEmitter>);

    /// Register a task on the scheduler, returning an index pointing to the task
    async fn register(self: &Arc<Self>, task: Task) -> usize;

    /// Remove (cancel) a task from the scheduler via an index
    async fn cancel(self: &Arc<Self>, index: usize);

    /// Abort the scheduler (shut it down) from any process it currently does
    async fn abort(self: &Arc<Self>);

    /// Clear all the scheduler's tasks completely
    async fn clear(self: &Arc<Self>);
}

/// This is the default implementation of a [`Scheduler`], the scheduler internally holds a map
/// of all indexes to task entries and a min-heap sorted based on the execution time (from earliest
/// to latest). The pipeline of the scheduler goes as follows:
///
/// - It pops a task off the min-heap, retrieving it in the process.
/// - If the task is marked for delete (lazy deletion), then it skips it entirely, if not
/// then continue with the pipeline.
/// - It takes the future time and converts it into an Instant.
/// - It sleeps til it hits the specific time (from the Instant).
/// - It modifies information such as the number of runs and the last execution time.
/// - It executes the task based on the overlap policy defined for that task.
/// - If the task reached its maximum runs then it stops for that task, if not then
/// continue with the pipeline.
/// - The scheduler calculates the new future time by parsing the schedule and using the time of
/// execution, then once it is done, stores that time to the task entry (to later retrieve it again).
/// - Re-allocates the task entry back to the min-heap (which is sorted based on the new future time).
/// - Repeat the process if there are any tasks left.
pub struct ChronologScheduler {
    earliest_sorted: Mutex<BinaryHeap<Reverse<ScheduledItem>>>,
    tasks: RwLock<IntMap<usize, Arc<Task>>>,
    task_process: ArcSwapOption<JoinHandle<()>>,
}

impl ChronologScheduler {
    /// Creates a brand-new default scheduler implementation instance
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tasks: RwLock::new(IntMap::default()),
            earliest_sorted: Mutex::new(BinaryHeap::new()),
            task_process: ArcSwapOption::from_pointee(None)
        })
    }
}

#[async_trait]
impl Scheduler for ChronologScheduler {
    async fn main(self: &'static Arc<Self>, emitter: Arc<TaskEventEmitter>) {
        let this = self.clone();
        self.task_process.store(Some(Arc::new(
            tokio::spawn(async move {
                loop {
                    let mut heap = this.earliest_sorted.lock().await;
                    if let Some(Reverse(scheduled_item)) = heap.pop() {
                        drop(heap);
                        let tasks_lock = self.tasks.read().await;
                        let task = tasks_lock.get(&scheduled_item.0).unwrap().clone();
                        drop(tasks_lock);
                        let (schedule, last_exec) = {
                            let now_chrono = Local::now();
                            let now_tokio = Instant::now();
                            let delta = &scheduled_item.1.signed_duration_since(now_chrono);
                            let target_time = if delta.num_milliseconds() <= 0 {
                                now_tokio
                            } else {
                                now_tokio + Duration::from_millis(delta.num_milliseconds() as u64)
                            };
                            sleep_until(target_time).await;
                            let max_runs = task.metadata.max_runs();
                            let policy = task.overlap_policy();
                            let last_exec = policy.handle(task.clone(), emitter.clone()).await;
                            task.metadata.last_execution().swap(Arc::new(last_exec));
                            let runs = task.metadata.runs().load(std::sync::atomic::Ordering::Relaxed);
                            match max_runs {
                                Some(m) if runs == m.get() => {continue},
                                _ => {}
                            };
                            (task.schedule(), last_exec)
                        };
                        let mut future_time = schedule.next_after(&last_exec).unwrap();
                        if (future_time - last_exec).subsec_millis() < 5 {
                            future_time = schedule.next_after(&(future_time + chrono::Duration::milliseconds(5))).unwrap();
                        }
                        let mut heap = this.earliest_sorted.lock().await;
                        heap.push(Reverse(ScheduledItem(scheduled_item.0, future_time)));
                        drop(heap);
                    }
                }
            })
        )));
    }

    async fn register(self: &Arc<Self>, task: Task) -> usize {
        let last_exec = Local::now();
        let future_time = task.schedule().next_after(&last_exec).unwrap();
        let id: usize = {
            let mut tasks = self.tasks.write().await;
            let id = tasks.len();
            tasks.insert(id, Arc::new(task));
            id
        };
        let entry = ScheduledItem(id, future_time);
        let mut earliest_tasks = self.earliest_sorted.lock().await;
        earliest_tasks.push(Reverse(entry));

        id
    }

    async fn cancel(self: &Arc<Self>, id: usize) {
        let mut tasks = self.tasks.write().await;
        tasks.remove(&id);
    }

    async fn abort(self: &Arc<Self>) {
        if let Some(process) = self.task_process.swap(None) {
            process.abort();
        }
    }

    async fn clear(self: &Arc<Self>) {
        self.earliest_sorted.lock().await.clear();
        self.tasks.write().await.clear();
    }
}