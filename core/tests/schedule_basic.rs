// core/tests/schedule_basic.rs
use chronolog_core::schedule::Schedule; // adjust the import path as needed

#[test]
fn test_next_run_interval() {
    // Fake start time
    let start = std::time::SystemTime::now();

    // Example: schedule repeating every 1 second
    let sched = Schedule::every_seconds(1);

    let next = sched.next_after(start).unwrap();

    // Assertion: next should be start + 1 second
    assert!(next.duration_since(start).unwrap().as_secs() == 1);
}
