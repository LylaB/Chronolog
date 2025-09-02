use std::num::NonZeroU64;
use std::time::Duration;
use chrono::{Local, Timelike};
use chronolog::scheduler::{Scheduler, CHRONOLOG_SCHEDULER};
use chronolog::task::execution::ExecutionTask;
use chronolog::schedule::{FieldSchedule, Schedule};
use chronolog::task_fn;

#[tokio::main]
async fn main() {
    let now  = Local::now();
    println!("START: {}.{}s", now.second(), now.timestamp_subsec_millis());
    let taskB = ExecutionTask::builder()
        .max_runs(NonZeroU64::new(30u64).unwrap())
        .schedule(Schedule::Every(Duration::from_secs(2)))
        .func(
            task_fn!(_metadata => {
                let now  = Local::now();
                println!("Task B: PROCESS - {}.{}s", now.second(), now.timestamp_subsec_millis());
                tokio::time::sleep(Duration::from_secs(4)).await;
                let now  = Local::now();
                println!("Task B: COMPLETED - {}.{}s", now.second(), now.timestamp_subsec_millis());
                Ok::<(), ()>(())
            })
        )
        .build();

    CHRONOLOG_SCHEDULER.register(taskB).await;
    CHRONOLOG_SCHEDULER.start().await;
    loop {}
}