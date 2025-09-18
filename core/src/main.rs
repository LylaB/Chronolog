use chrono::{Local, Timelike};
use chronolog_core::schedule::TaskScheduleInterval;
use chronolog_core::scheduler::CHRONOLOG_SCHEDULER;
use chronolog_core::task::{DefaultTaskMetadata, ExecutionTaskFrame, Task};

#[tokio::main]
async fn main() {
    let now = Local::now();
    println!(
        "PROCESS STARTED {}.{}s",
        now.second(),
        now.timestamp_subsec_millis()
    );

    let frame1 = ExecutionTaskFrame::new(|_| async {
        let now = Local::now();
        println!(
            "Task A Frame Executed At {}.{}s",
            now.second(),
            now.timestamp_subsec_millis()
        );
        Ok(())
    });
    let frame2 = ExecutionTaskFrame::new(|_| async {
        let now = Local::now();
        println!(
            "Task B Frame Executed At {}.{}s",
            now.second(),
            now.timestamp_subsec_millis()
        );
        Ok(())
    });
    CHRONOLOG_SCHEDULER
        .schedule(
            Task::builder()
                .frame(frame2)
                .metadata(DefaultTaskMetadata::new_with("Task B".to_owned()))
                .schedule(TaskScheduleInterval::from_secs(1))
                .build(),
        )
        .await;
    CHRONOLOG_SCHEDULER
        .schedule(
            Task::builder()
                .frame(frame1)
                .metadata(DefaultTaskMetadata::new_with("Task A".to_owned()))
                .schedule(TaskScheduleInterval::from_secs(1))
                .build(),
        )
        .await;
    CHRONOLOG_SCHEDULER.start().await;
    loop {}
}
