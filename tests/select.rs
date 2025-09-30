use eee499_timer::{runtime, select, timer};
use futures::future::Future;
use runtime::{Runtime, Task};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use timer::SleepFuture;

#[test]
fn test_select() {
    let f1 = SleepFuture::new(std::time::Duration::new(1, 0));
    let f2 = SleepFuture::new(std::time::Duration::new(2, 0));
    let select = select::select(f1, f2);
    let start = Instant::now();
    let executor = Runtime::new();

    executor
        .spawn(async {
            assert!(
                matches!(select.await, select::Either::A(_)),
                "select should have returned first future"
            );
        })
        .expect("failed to spawn");
    executor.run();
    let run_time = start.elapsed();
    assert!(
        run_time >= Duration::from_secs(1),
        "select should have waited for first future"
    );
    assert!(
        run_time <= Duration::from_secs(2),
        "select should not have waited for second future"
    );
}
