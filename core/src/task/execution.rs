use std::error::Error;
use crate::task::{Arc};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use typed_builder::TypedBuilder;
use crate::errors::ChronologErrors;
use crate::task::{Schedule, Task};
use once_cell::sync::Lazy;
use crate::overlap::{OverlapStrategy, SequentialOverlapPolicy};

static EXECUTION_TASK_CREATION_COUNT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

#[derive(TypedBuilder)]
#[builder(build_method(into = ExecutionTask<E, F>))]
pub struct ExecutionTaskConfig<E, F>
where
    ExecutionTask<E, F>: From<ExecutionTaskConfig<E, F>>,
    E: Send + Sync + Debug + 'static,
    F: (Fn(ExecutionTaskMetadata) -> Pin<Box<dyn Future<Output = Result<(), E>> + Send>>) + Send + Sync
{
    func: F,
    
    #[builder(setter(transform = |s: impl Schedule + 'static| Arc::new(s) as Arc<dyn Schedule>))]
    schedule: Arc<dyn Schedule>,

    #[builder(default, setter(strip_option))]
    max_runs: Option<NonZeroU64>,

    #[builder(default, setter(strip_option))]
    debug_label: Option<String>,

    #[builder(default, setter(skip))]
    _marker: PhantomData<E>,

    #[builder(default = Arc::new(SequentialOverlapPolicy), setter(transform = |s: impl OverlapStrategy + 'static| Arc::new(s) as Arc<dyn OverlapStrategy>))]
    overlap_policy: Arc<dyn OverlapStrategy>,
}

impl<E, F> From<ExecutionTaskConfig<E, F>> for ExecutionTask<E, F>
where
    E: Send + Sync + Debug + 'static,
    F: (Fn(ExecutionTaskMetadata) -> Pin<Box<dyn Future<Output = Result<(), E>> + Send>>) + Send + Sync
{
    fn from(config: ExecutionTaskConfig<E, F>) -> Self {
        let creation_time = Local::now();
        let debug_label = if let Some(debug_label) = config.debug_label {
            debug_label
        } else {
            let num = EXECUTION_TASK_CREATION_COUNT.fetch_add(1, Ordering::Relaxed);
            format!("ExecutionTask#{}", num)
        };
        Self {
            func: config.func,
            metadata: ExecutionTaskMetadata {
                schedule: config.schedule,
                runs: 0,
                max_runs: config.max_runs,
                debug_label,
                overlap_policy: config.overlap_policy,
            },
            last_execution: ArcSwap::from_pointee(creation_time),
            _marker: PhantomData::default()
        }
    }
}

pub struct ExecutionTask<E, F>
where
    E: Send + Sync + Debug + 'static,
    F: (Fn(ExecutionTaskMetadata) -> Pin<Box<dyn Future<Output = Result<(), E>> + Send>>) + Send + Sync
{
    func: F,
    metadata: ExecutionTaskMetadata,
    last_execution: ArcSwap<DateTime<Local>>,
    _marker: PhantomData<E>
}

pub struct ExecutionTaskMetadata {
    pub schedule: Arc<dyn Schedule>,
    pub runs: u64,
    pub max_runs: Option<NonZeroU64>,
    pub debug_label: String,
    pub overlap_policy: Arc<dyn OverlapStrategy>
}

impl<E, F> ExecutionTask<E, F>
where
    E: Send + Sync + Debug + 'static,
    F: (Fn(ExecutionTaskMetadata) -> Pin<Box<dyn Future<Output = Result<(), E>> + Send>>) + Send + Sync
{
    pub fn builder() -> ExecutionTaskConfigBuilder<E, F> {
        ExecutionTaskConfig::builder()
    }
}

#[async_trait]
impl<E, F> Task for ExecutionTask<E, F>
where
    E: Send + Sync + Debug + 'static,
    F: (Fn(ExecutionTaskMetadata) -> Pin<Box<dyn Future<Output = Result<(), E>> + Send>>) + Send + Sync
{
    async fn execute_inner(&self) -> Result<(), Arc<dyn Error + Send + Sync>> {
        let cloned_metadata = ExecutionTaskMetadata {
            schedule: self.metadata.schedule.clone(),
            runs: self.metadata.runs,
            max_runs: self.metadata.max_runs,
            overlap_policy: self.metadata.overlap_policy.clone(),
            debug_label: self.metadata.debug_label.clone(),
        };
        let result = (self.func)(cloned_metadata).await;
        if let Err(err) = result {
            return Err(Arc::new(ChronologErrors::FailedExecution(
                self.get_debug_label().await, Box::new(err))
            ));
        }
        self.last_execution.swap(Arc::new(Local::now()));
        Ok(())
    }

    async fn get_schedule(&self) -> Arc<dyn Schedule> {
        self.metadata.schedule.clone()
    }

    async fn total_runs(&self) -> u64 {
        self.metadata.runs
    }

    async fn maximum_runs(&self) -> Option<NonZeroU64> {
        self.metadata.max_runs
    }

    async fn set_maximum_runs(&mut self, max_runs: NonZeroU64) {
        self.metadata.max_runs = Some(max_runs)
    }

    async fn set_total_runs(&mut self, runs: u64) {
        self.metadata.runs = runs;
    }

    async fn set_last_execution(&mut self, exec: DateTime<Local>) {
        self.last_execution.swap(Arc::new(exec));
    }

    async fn get_debug_label(&self) -> String {
        self.metadata.debug_label.clone()
    }

    async fn last_execution(&self) -> DateTime<Local> {
        *self.last_execution.load().clone()
    }

    async fn overlap_policy(&self) -> Arc<dyn OverlapStrategy> {
        self.metadata.overlap_policy.clone()
    }
}