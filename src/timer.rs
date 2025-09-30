use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

pub struct SleepFuture {
    state: SleepState,
}
enum SleepState {
    /// the future is created but not yet polled
    Created(Duration),
    /// the future is currently waiting for the timer to complete
    Running(Arc<Mutex<SleepContext>>),
    /// the future has completed
    Done,
}

struct SleepContext {
    shared_waker: Option<Waker>,
    completed: bool,
}

impl SleepFuture {
    /// Create a new `SleepFuture` which will complete after a timeout
    pub fn new(duration: Duration) -> Self {
        SleepFuture {
            state: SleepState::Created(duration),
        }
    }

    fn spawn_timer_thread(mut self: Pin<&mut Self>, cx: &mut Context, duration: Duration) {
        let context = SleepContext {
            shared_waker: Some(cx.waker().clone()),
            completed: false,
        };
        let context: Arc<Mutex<SleepContext>> = Arc::new(Mutex::new(context));
        let clone_for_thread = context.clone();
        let _ = thread::spawn(move || {
            todo!(
                "pause the thread until the duration is completed. \
            When the thread resumes call wake if the wake function exists"
            );
        });
        self.state = SleepState::Running(context);
    }
}

impl Future for SleepFuture {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            SleepState::Created(duration) => {
                todo!("spawn the timer thread");
            }
            SleepState::Running(ref ctx) => {
                todo!(
                    "check if the thread is done. If it is, set the state to Done and return Poll::Ready(()). \
                if not update the shared wake function and return Poll::Pending."
                );
            }
            SleepState::Done => Poll::Ready(()),
        }
    }
}

impl Drop for SleepFuture {
    fn drop(&mut self) {
        if let SleepState::Running(context) = &self.state {
            todo!("remove the context from the timer thread to free its resources");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ntest::timeout;
    use std::time::{Duration, Instant};

    #[test]
    #[timeout(3000)]
    fn test_timer_future() {
        // this will pass as long as the duration in the future is correctly waited for
        let start = Instant::now();
        let timer = SleepFuture::new(Duration::from_secs(2));
        futures::executor::block_on(timer);
        let run_time = start.elapsed();
        assert!(run_time >= Duration::from_secs(2));
        assert!(run_time - Duration::from_secs(2) < Duration::from_millis(100));
    }

    #[test]
    #[timeout(10000)]
    fn test_timers_parallel() {
        let start = Instant::now();
        let timers: Vec<_> = (0..10)
            .map(|_| SleepFuture::new(Duration::from_secs(2)))
            .collect();
        // this is just going to poll them all in a loop - will pass as long as the futures don't block
        futures::executor::block_on(futures::future::join_all(timers));
        let run_time = start.elapsed();
        assert!(run_time >= Duration::from_secs(2));
        assert!(run_time - Duration::from_secs(2) < Duration::from_millis(100));
    }

    #[test]
    #[timeout(10000)]
    fn test_timers_start() {
        let start = Instant::now();
        let timer = SleepFuture::new(Duration::from_millis(200));
        thread::sleep(Duration::from_millis(100));
        futures::executor::block_on(timer);
        let run_time = start.elapsed();
        assert!(run_time >= Duration::from_millis(300));
    }
}
