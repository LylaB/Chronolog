use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Arc};
use std::time::Duration;
use arc_swap::{ArcSwapOption};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Local, LocalResult, NaiveDate, NaiveTime, TimeZone, Timelike};
use nohash_hasher::IntMap;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Instant};
use crate::task::{FieldSchedule, Schedule, Task};

pub static CHRONOLOG_SCHEDULER: Lazy<Arc<ChronologScheduler>> = Lazy::new(|| ChronologScheduler::new());

#[async_trait]
pub trait Scheduler {
    async fn start(self: Arc<Self>);
    async fn next_task(self: Arc<Self>) -> Option<Arc<dyn Task>>;
    async fn register(self: Arc<Self>, task: impl Task + 'static) -> usize;
    async fn cancel(self: Arc<Self>, index: usize) -> Option<Arc<dyn Task>>;
}

pub struct ChronologScheduler {
    reference: Mutex<IntMap<usize, Arc<Mutex<dyn Task>>>>,
    earliest: Mutex<BinaryHeap<Reverse<(DateTime<Local>, usize)>>>,
    task_process: ArcSwapOption<JoinHandle<()>>,
}

fn last_day_of_month(year: i32, month: u32) -> u32 {
    (NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - chrono::Duration::seconds(86400)).day()
}

fn rebuild_datetime_from_parts(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: u32,
) -> DateTime<Local> {
    let day = std::cmp::min(day, last_day_of_month(year, month));
    let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
    let time = NaiveTime::from_hms_milli_opt(hour, minute, second, millisecond).unwrap();
    let naive = date.and_time(time);

    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(dt1, _dt2) => dt1,
        LocalResult::None => {
            for minutes in 1..=120 {
                let candidate = naive + chrono::Duration::minutes(minutes);
                if let LocalResult::Single(dt) = Local.from_local_datetime(&candidate) {
                    return dt;
                }
            }
            chrono::Utc.from_utc_datetime(&naive).with_timezone(&Local)
        }
    }
}

macro_rules! parse_schedule_field {
    ($field: expr, $time: expr, $mult: expr, $time_unit: tt) => {{
        match $field {
            FieldSchedule::Ignore => {}
            FieldSchedule::Every(res) => {
                $time += Duration::from_secs_f64(res.clone() as f64 * $mult)}
            FieldSchedule::Exactly(res) => {
                let mut parts = (
                    $time.year() as u32,
                    $time.month0() as u8,
                    $time.day0() as u8,
                    $time.hour() as u8,
                    $time.minute() as u8,
                    $time.second() as u8,
                    $time.timestamp_subsec_millis() as u16
                );

                parts.$time_unit = res.clone().into();

                $time = rebuild_datetime_from_parts(
                    parts.0 as i32,
                    parts.1 as u32,
                    parts.2 as u32,
                    parts.3 as u32,
                    parts.4 as u32,
                    parts.5 as u32,
                    parts.6 as u32
                );
            }
            FieldSchedule::Custom(custom) => {
                let mut parts = (
                    $time.year() as u32,
                    $time.month0() as u8,
                    $time.day0() as u8,
                    $time.hour() as u8,
                    $time.minute() as u8,
                    $time.second() as u8,
                    $time.timestamp_subsec_millis() as u16
                );

                parts.$time_unit = custom(parts.$time_unit.clone());

                $time = rebuild_datetime_from_parts(
                    parts.0 as i32,
                    parts.1 as u32,
                    parts.2 as u32,
                    parts.3 as u32,
                    parts.4 as u32,
                    parts.5 as u32,
                    parts.6 as u32
                );
            }
        }
    }};
}

fn run_when(schedule: &Schedule) -> DateTime<Local> {
    match schedule {
        Schedule::Every(d) => { Local::now() + *d }
        Schedule::Calendar {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond
        } => {
            let mut now = Local::now();
            parse_schedule_field!(year, now, 3.154e+7, 0);
            parse_schedule_field!(month, now, 2.628e+6, 1);
            parse_schedule_field!(day, now, 86400.0, 2);
            parse_schedule_field!(hour, now, 3600.0, 3);
            parse_schedule_field!(minute, now, 60.0, 4);
            parse_schedule_field!(second, now, 1.0, 5);
            parse_schedule_field!(millisecond, now, 0.001, 6);
            now
        }
        Schedule::Cron(_) => {
            todo!()
        }
    }
}

impl ChronologScheduler {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            earliest: Mutex::new(BinaryHeap::new()),
            reference: Mutex::new(IntMap::default()),
            task_process: ArcSwapOption::from_pointee(None)
        })
    }

    async fn refresh(self: &Arc<Self>) {
        if let Some(process) = self.task_process.swap(None) {
            process.abort()
        }
        if self.reference.lock().await.is_empty() {
            return;
        }
        let (id, task, time) = self.get_next().await.unwrap();
        let this = Arc::clone(self);
        self.task_process.swap(Some(Arc::new(
            tokio::spawn(async move {
                let now_chrono = Local::now();
                let now_tokio = Instant::now();

                let delta = time.signed_duration_since(now_chrono);
                let target_time = if delta.num_milliseconds() <= 0 {
                    now_tokio
                } else {
                    now_tokio + Duration::from_millis(delta.num_milliseconds() as u64)
                };
                sleep_until(target_time).await;
                {
                    let mut lock = task.lock().await;
                    lock.execute().await.unwrap();
                    let runs = lock.total_runs().await;
                    lock.set_total_runs(runs + 1).await;
                }

                tokio::spawn(async move {
                    this.clone().refresh().await;
                });
            })
        )));
    }

    async fn get_next(&self) -> Option<(usize, Arc<Mutex<dyn Task>>, DateTime<Local>)> {
        let mut heap = self.earliest.lock().await;
        let tasks = self.reference.lock().await;

        while let Some(Reverse((when, id))) = heap.peek().cloned() {
            if let Some(task) = tasks.get(&id) {
                return Some((id, task.clone(), when));
            } else {
                heap.pop();
            }
        }
        None
    }
}

#[async_trait]
impl Scheduler for ChronologScheduler {
    async fn start(self: Arc<Self>) {
        self.refresh().await;
    }

    async fn next_task(self: Arc<Self>) -> Option<Arc<dyn Task>> {
        todo!()
    }

    async fn register(self: Arc<Self>, task: impl Task + 'static) -> usize {
        let task = Arc::new(Mutex::new(task));
        let id: usize = {
            let mut tasks = self.reference.lock().await;
            let id = tasks.len();
            tasks.insert(id, task.clone());
            id
        };

        let task_lock = task.lock().await;
        let schedule = task_lock.get_schedule().await;
        let when = run_when(schedule);
        {
            let mut heap = self.earliest.lock().await;
            heap.push(Reverse((when, id)));
        }

        self.refresh().await;
        id
    }

    async fn cancel(self: Arc<Self>, id: usize) -> Option<Arc<dyn Task>> {
        let result = {
            let mut tasks = self.reference.lock().await;
            tasks.remove(&id)
        };

        self.refresh().await;
        todo!()
    }
}