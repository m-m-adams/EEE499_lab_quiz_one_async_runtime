use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};
use std::sync::MutexGuard;

pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

/// Shared state between the future and the waiting thread. You will be able to pass a clone of
/// the Arc<Mutex<SharedState>> to another thread, allowing that thread to wake the future on completion
struct SharedState {
    /// Whether or not the sleep time has elapsed
    completed: bool,

    /// The waker for the task that `TimerFuture` is running on.
    /// The thread can use this after setting `completed = true` to tell
    /// `TimerFuture`'s task to wake up, see that `completed = true`, and
    /// move forward.
    waker: Option<Waker>,
}

impl Future for TimerFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Lock the shared state, check if the timer has completed, and set the waker if it hasn't
        let mut shared_state: MutexGuard<SharedState> = todo!();
        if shared_state.completed {
            // return the correct value for a completed future that is ready to continue
            todo!()
        } else {
            // Set waker so that the thread can wake up the current task
            // when the timer has completed, ensuring that the future is polled
            // again and sees that `completed = true`.
            //
            // the waker can be extracted from the current task context with cx.waker().clone()
            // must be done on every poll as the waker can change if the task was moved to a new executor
            todo!()
        }
    }
}

impl TimerFuture {
    /// Create a new `TimerFuture` which will complete after the provided
    /// timeout.
    pub fn new(duration: Duration) -> Self {
        let shared_state: Arc<Mutex<SharedState>> = todo!();

        // Spawn the new thread
        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            // sleep the thread until the duration is completed, then call wake
            todo!();
            // when the thread resumes
            // Signal that the timer has completed and
            // call the wake function if one exists.
            let mut shared_state: MutexGuard<SharedState> = todo!();
            todo!();
        });

        TimerFuture { shared_state }
    }
}

#[cfg(test)]
mod tests {
    use super::TimerFuture;
    use std::time::{Duration, Instant};

    #[test]
    fn test_timer_future() {
        // this will pass as long as the duration in the future is correctly waited for
        let start = Instant::now();
        let timer = TimerFuture::new(Duration::from_secs(2));
        futures::executor::block_on(timer);
        let run_time = start.elapsed();
        assert!(run_time >= Duration::from_secs(2));
        assert!(run_time - Duration::from_secs(2) < Duration::from_millis(100));
    }

    #[test]
    fn test_timers_parallel() {
        let start = Instant::now();
        let timers: Vec<_> = (0..10)
            .map(|_| TimerFuture::new(Duration::from_secs(2)))
            .collect();
        // this is just going to poll them all in a loop - will pass as long as the futures don't block
        futures::executor::block_on(futures::future::join_all(timers));
        let run_time = start.elapsed();
        assert!(run_time >= Duration::from_secs(2));
        assert!(run_time - Duration::from_secs(2) < Duration::from_millis(100));
    }
}
