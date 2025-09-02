use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use typed_builder::TypedBuilder;
use crate::task::{Schedule, Task};
use once_cell::sync::Lazy;

static PARALLEL_TASK_CREATION_COUNT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

pub enum ParallelTaskPolicy {
    RunSilenceFailures,
    RunUntilSuccess,
    RunUntilFailure,
}

#[derive(TypedBuilder)]
#[builder(build_method(into = ParallelTask))]
pub struct ParallelTaskConfig
where ParallelTask: From<ParallelTaskConfig> {
    tasks: Vec<Arc<dyn Task>>,
    schedule: Schedule,

    #[builder(default, setter(strip_option))]
    max_runs: Option<u64>,

    #[builder(default, setter(strip_option))]
    debug_label: Option<String>,

    #[builder(default = ParallelTaskPolicy::RunUntilFailure)]
    policy: ParallelTaskPolicy,
}

impl From<ParallelTaskConfig> for ParallelTask {
    fn from(config: ParallelTaskConfig) -> Self {
        let creation_time = Local::now();
        let debug_label = if let Some(debug_label) = config.debug_label {
            debug_label
        } else {
            let num = PARALLEL_TASK_CREATION_COUNT.fetch_add(1, Ordering::Relaxed);
            format!("ParallelTask#{}", num)
        };
        Self {
            tasks: config.tasks,
            schedule: config.schedule,
            runs: 0,
            max_runs: config.max_runs,
            debug_label,
            policy: config.policy,
            last_execution: ArcSwap::from_pointee(creation_time)
        }
    }
}

pub struct ParallelTask {
    tasks: Vec<Arc<dyn Task>>,
    schedule: Schedule,
    runs: u64,
    max_runs: Option<u64>,
    debug_label: String,
    policy: ParallelTaskPolicy,
    last_execution: ArcSwap<DateTime<Local>>
}

impl ParallelTask {
    pub fn builder() -> ParallelTaskConfigBuilder {
        ParallelTaskConfig::builder()
    }
}

#[async_trait]
impl Task for ParallelTask {
    async fn execute(&self) -> Result<(), Arc<dyn Error + Send + Sync>> {
        let mut futures = FuturesUnordered::from_iter(
            self.tasks.iter().map(|x| x.execute())
        );

        while let Some(result) = futures.next().await {
            match (&self.policy, result) {
                (ParallelTaskPolicy::RunUntilFailure, Err(res)) => {
                    return Err(res)
                }
                (ParallelTaskPolicy::RunUntilSuccess, Ok(res)) => {
                    return Ok(res);
                }
                (_, Ok(_)) => {}
                (_, Err(_)) => {}
            }
        }
        self.last_execution.store(Arc::new(Local::now()));
        Ok(())
    }

    async fn get_schedule(&self) -> &Schedule {
        &self.schedule
    }

    async fn total_runs(&self) -> u64 {
        self.runs
    }

    async fn maximum_runs(&self) -> Option<u64> {
        self.max_runs
    }

    async fn set_total_runs(&mut self, runs: u64) {
        self.runs = runs;
    }

    async fn get_debug_label(&self) -> String {
        self.debug_label.clone()
    }

    async fn last_execution(&self) -> DateTime<Local> {
        *self.last_execution.load().clone()
    }
}