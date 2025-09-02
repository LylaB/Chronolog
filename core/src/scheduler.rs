use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::sync::{Arc};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use arc_swap::{ArcSwap, ArcSwapOption};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Local, LocalResult, Months, NaiveDate, TimeZone, Timelike};
use nohash_hasher::IntMap;
use once_cell::sync::Lazy;
use tokio::sync::{Mutex};
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Instant};
use tokio_util::sync::CancellationToken;
use crate::overlap::OverlapContext;
use crate::schedule::{FieldSchedule, Schedule};
use crate::task::Task;

pub static CHRONOLOG_SCHEDULER: Lazy<Arc<ChronologScheduler>> = Lazy::new(|| ChronologScheduler::new());

/// The [`Scheduler`] trait defines a basis for what a scheduler is, a scheduler is the actual mechanism
/// that drives the execution of multiple [`Task`] structs in specific times the tasks request,
/// typically this is mainly for extensibility. In most scenarios, the default global scheduler
/// which is [`CHRONOLOG_SCHEDULER`] or the struct [`ChronologScheduler`] can be used for the scheduling logic
///
/// All methods are async and in addition, require `&Arc<Self>` as opposed to typical `&self`,
/// for multi-thread and ergonomic reasons
///
/// Schedulers do not start by default, they explicitly require the developer to call [`Scheduler::start`] to
/// start it, schedulers can also be aborted via the method [`Scheduler::abort`]
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
pub trait Scheduler {
    async fn start(self: &Arc<Self>);
    async fn register(self: &Arc<Self>, task: impl Task + 'static) -> usize;
    async fn cancel(self: &Arc<Self>, index: usize);
    async fn abort(self: &Arc<Self>);
    async fn clear(self: &Arc<Self>);
}

pub(crate) struct TaskEntry {
    pub(crate) task: Arc<Mutex<dyn Task>>,
    pub(crate) target_time: ArcSwap<DateTime<Local>>,
    pub(crate) marked_for_delete: AtomicBool,
    pub(crate) process: ArcSwapOption<JoinHandle<()>>,
    pub(crate) cancel_token: ArcSwapOption<CancellationToken>,
}

impl Eq for TaskEntry {}

impl PartialEq<Self> for TaskEntry {
    fn eq(&self, other: &Self) -> bool {
        *self.target_time.load() == *other.target_time.load()
    }
}

impl PartialOrd<Self> for TaskEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.target_time.load().partial_cmp(&other.target_time.load())
    }
}

impl Ord for TaskEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.target_time.load().cmp(&other.target_time.load())
    }
}
/// This is the default implementation of a [`Scheduler`], the scheduler internally holds a map
/// of all indexes to task entries and a min-heap sorted based on the execution time (from earliest
/// to latest). The pipeline of the scheduler goes as follows:
/// - It pops a task off the min-heap, retrieving it in the process
/// - If the task is marked for delete (lazy deletion), then it skips it entirely, if not
/// then continue with the pipeline
/// - It takes the future time and converts it into an Instant
/// - It sleeps til it hits the specific time (from the Instant)
/// - It modifies information such as the number of runs and the last execution time
/// - It executes the task based on the overlap policy defined for that task
/// - If the task reached its maximum runs then it stops for that task, if not then
/// continue with the pipeline
/// - The scheduler calculates the new future time by parsing the schedule and using the time of
/// execution, then once it is done, stores that time to the task entry (to later retrieve it again)
/// - Re-allocates the task entry back to the min-heap (which is sorted based on the new future time)
/// - Repeats the process if there are any tasks
pub struct ChronologScheduler {
    earliest_sorted: Mutex<BinaryHeap<Reverse<Arc<TaskEntry>>>>,
    tasks: Mutex<IntMap<usize, Arc<TaskEntry>>>,
    task_process: ArcSwapOption<JoinHandle<()>>,
}

#[inline(always)]
fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    (NaiveDate::from_ymd_opt(next_year, next_month, 1).unwrap() - chrono::Duration::days(1)).day()
}

#[inline]
fn rebuild_datetime_from_parts(
    year: i32, month: u32, day: u32,
    hour: u32, minute: u32, second: u32, millisecond: u32
) -> DateTime<Local> {
    let day = std::cmp::min(day, last_day_of_month(year, month));
    let naive = NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_milli_opt(hour, minute, second, millisecond)
        .unwrap();

    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(dt1, _) => dt1,
        LocalResult::None => {
            let mut candidate = naive;
            for _ in 0..10 {  // Usually DST gaps are max 1-2 hours
                candidate += chrono::Duration::minutes(1);
                if let LocalResult::Single(dt) = Local.from_local_datetime(&candidate) {
                    return dt;
                }
            }
            chrono::Utc.from_utc_datetime(&naive).with_timezone(&Local)
        }
    }
}


#[inline]
async fn run_when(last_exec: DateTime<Local>, schedule: &Schedule) -> DateTime<Local> {
    match schedule {
        Schedule::Every(d) => { last_exec + *d }
        Schedule::Calendar {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond
        } => {
            let mut dates = [
                last_exec.timestamp_subsec_millis(), last_exec.second(),
                last_exec.minute(), last_exec.hour(), last_exec.day0(),
                last_exec.month0(), last_exec.year() as u32
            ];
            let fields = [millisecond, second, minute, hour, day, month, year];
            for (index, &field) in fields.iter().enumerate() {
                let date_field = dates.get_mut(index).unwrap();
                match field {
                    FieldSchedule::Ignore => {}
                    FieldSchedule::Every(d) => {
                        *date_field += (*d as f64)
                            .clamp(u32::MIN as f64, u32::MAX as f64)
                            .round() as u32;
                    }
                    FieldSchedule::Exactly(res) => {
                        *date_field = *res;
                    }
                    FieldSchedule::Custom(func) => {
                        *date_field = func(*date_field);
                    }
                }
            }
            let mut modified = rebuild_datetime_from_parts(
                dates[6] as i32,
                dates[5] + 1,
                dates[4] + 1,
                dates[3],
                dates[2],
                dates[1],
                dates[0]
            );
            for (index, &field) in fields.iter().enumerate() {
                if index > 0 && matches!(fields[index - 1], FieldSchedule::Exactly(_))
                    && !matches!(field, FieldSchedule::Exactly(_)) && modified < last_exec {
                    match index {
                        0 => {},
                        1 => modified += chrono::Duration::seconds(1),
                        2 => modified += chrono::Duration::minutes(1),
                        3 => modified += chrono::Duration::hours(1),
                        4 => modified += chrono::Duration::days(1),
                        5 => modified = modified + Months::new(1),
                        6 => modified = modified + Months::new(12),
                        _ => {}
                    };
                }
            }
            modified
        }
        Schedule::Cron(expr) => {
            cron_parser::parse(expr, &last_exec).unwrap()
        }
    }
}

impl ChronologScheduler {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tasks: Mutex::new(IntMap::default()),
            earliest_sorted: Mutex::new(BinaryHeap::new()),
            task_process: ArcSwapOption::from_pointee(None)
        })
    }
}

#[async_trait]
impl Scheduler for ChronologScheduler {
    async fn start(self: &Arc<Self>) {
        let this = self.clone();
        self.task_process.store(Some(Arc::new(
            tokio::spawn(async move {
                let mut heap = this.earliest_sorted.lock().await;
                while let Some(Reverse(task_entry)) = heap.pop() {
                    if task_entry.marked_for_delete.load(std::sync::atomic::Ordering::Relaxed) {
                        continue;
                    }
                    let (schedule, last_exec) = {
                        let loaded_time = task_entry.target_time.load();
                        let task_lock = task_entry.task.lock().await;
                        let now_chrono = Local::now();
                        let now_tokio = Instant::now();
                        let delta = &loaded_time.signed_duration_since(now_chrono);
                        let target_time = if delta.num_milliseconds() <= 0 {
                            now_tokio
                        } else {
                            now_tokio + Duration::from_millis(delta.num_milliseconds() as u64)
                        };
                        drop(task_lock);
                        sleep_until(target_time).await;
                        let mut task_lock = task_entry.task.lock().await;
                        let runs = task_lock.total_runs().await + 1;
                        task_lock.set_total_runs(runs).await;
                        let last_exec = Local::now();
                        task_lock.set_last_execution(last_exec).await;
                        let max_runs = task_lock.maximum_runs().await;
                        let schedule = task_lock.get_schedule().await.clone();
                        let policy = task_lock.overlap_policy().await;
                        drop(task_lock);
                        policy.execute(&OverlapContext(&*task_entry)).await;
                        match max_runs {
                            Some(m) if runs == m.get() => {continue},
                            _ => {}
                        };
                        (schedule, last_exec)
                    };
                    let mut future_time = run_when(last_exec, &schedule).await;
                    if (future_time - last_exec).subsec_millis() < 5 {
                        future_time = run_when(future_time + chrono::Duration::milliseconds(5), &schedule).await;
                    }
                    task_entry.target_time.store(Arc::new(future_time));
                    heap.push(Reverse(task_entry));
                }
            })
        )));
    }

    async fn register(self: &Arc<Self>, task: impl Task + 'static) -> usize {
        let task = Arc::new(Mutex::new(task));
        let target_time = {
            let task_lock = task.lock().await;
            let last_exec = task_lock.last_execution().await;
            let schedule = task_lock.get_schedule().await;

            ArcSwap::from_pointee(run_when(last_exec, schedule).await)
        };
        let entry = Arc::new(TaskEntry {
            task,
            target_time,
            marked_for_delete: AtomicBool::new(false),
            cancel_token: ArcSwapOption::new(None),
            process: ArcSwapOption::new(None)
        });
        let mut earliest_tasks = self.earliest_sorted.lock().await;
        earliest_tasks.push(Reverse(entry.clone()));
        let id: usize = {
            let mut tasks = self.tasks.lock().await;
            let id = tasks.len();
            tasks.insert(id, entry);
            id
        };

        id
    }

    async fn cancel(self: &Arc<Self>, id: usize) {
        let mut tasks = self.tasks.lock().await;
        if let Some(registry) = tasks.remove(&id) {
            registry.marked_for_delete.store(true, std::sync::atomic::Ordering::Release);
        }
    }

    async fn abort(self: &Arc<Self>) {
        if let Some(process) = self.task_process.swap(None) {
            process.abort();
        }
    }

    async fn clear(self: &Arc<Self>) {
        self.earliest_sorted.lock().await.clear();
        self.tasks.lock().await.clear();
    }
}