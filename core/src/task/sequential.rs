use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use typed_builder::TypedBuilder;
use crate::task::{Schedule, Task};
use once_cell::sync::Lazy;

static SEQUENTIAL_TASK_CREATION_COUNT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

pub enum SequentialTaskPolicy {
    RunSilenceFailures,
    RunUntilSuccess,
    RunUntilFailure,
}

#[derive(TypedBuilder)]
#[builder(build_method(into = SequentialTask))]
pub struct SequentialTaskConfig
where SequentialTask: From<SequentialTaskConfig> {
    tasks: Vec<Arc<dyn Task>>,
    schedule: Schedule,

    #[builder(default, setter(strip_option))]
    max_runs: Option<u64>,

    #[builder(default, setter(strip_option))]
    debug_label: Option<String>,

    #[builder(default = SequentialTaskPolicy::RunUntilFailure)]
    policy: SequentialTaskPolicy,
}

impl From<SequentialTaskConfig> for SequentialTask  {
    fn from(config: SequentialTaskConfig) -> Self {
        let creation_time = Local::now();
        let debug_label = if let Some(debug_label) = config.debug_label {
            debug_label
        } else {
            let num = SEQUENTIAL_TASK_CREATION_COUNT.fetch_add(1, Ordering::Relaxed);
            format!("SequentialTask#{}", num)
        };
        Self {
            tasks: config.tasks,
            schedule: config.schedule,
            runs: 0,
            max_runs: config.max_runs,
            debug_label,
            policy: config.policy,
            last_execution: ArcSwap::new(Arc::new(creation_time)),
        }
    }
}

pub struct SequentialTask {
    tasks: Vec<Arc<dyn Task>>,
    schedule: Schedule,
    runs: u64,
    max_runs: Option<u64>,
    debug_label: String,
    policy: SequentialTaskPolicy,
    last_execution: ArcSwap<DateTime<Local>>
}

impl SequentialTask {
    pub fn builder() -> SequentialTaskConfigBuilder {
        SequentialTaskConfig::builder()
    }
}

#[async_trait]
impl Task for SequentialTask {
    async fn execute(&self) -> Result<(), Arc<dyn Error + Send + Sync>> {
        for task in self.tasks.iter() {
            match (&self.policy, task.execute().await) {
                (SequentialTaskPolicy::RunUntilFailure, Err(res)) => {
                    return Err(res)
                }
                (SequentialTaskPolicy::RunUntilSuccess, Ok(res)) => {
                    return Ok(res);
                }
                (_, Ok(_)) => {}
                (_, Err(_)) => {}
            }
        }
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