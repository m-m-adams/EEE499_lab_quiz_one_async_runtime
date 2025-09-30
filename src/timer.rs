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
enum SleepState{
    /// the future is created but not yet polled
    Created(Duration),
    /// the future is currently waiting for the timer to complete
    Running(SleepContext),
    /// the future has completed
    Done,
}

struct SleepContext {
    shared_waker: Arc<Mutex<Option<Waker>>>,
    waiting_thread: thread::JoinHandle<()>,
}

impl Future for SleepFuture {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &self.state {
            SleepState::Created(duration) => {
                self.state = SleepState::Running(self.spawn_wait_thread(*duration, cx));
                Poll::Pending
            }
            SleepState::Running(ctx) => {
                if ctx.waiting_thread.is_finished() {
                    self.state = SleepState::Done;
                    return Poll::Ready(());
                }

                let mut waker = ctx.shared_waker.lock().unwrap();
                *waker = Some(cx.waker().clone());
                Poll::Pending
            }
            SleepState::Done => {
                Poll::Ready(())
            }
        }

    }
}

impl SleepFuture {
    /// Create a new `TimerFuture` which will complete after the provided
    /// timeout.
    pub fn new(duration: Duration) -> Self {
        SleepFuture { state: SleepState::Created(duration) }
    }
    fn spawn_wait_thread(&self, duration: Duration, cx: &mut Context<'_>) ->  SleepContext {
        let waker = cx.waker().clone();
        let shared_waker = Arc::new(Mutex::new(Some(waker)));
        let cloned_waker = shared_waker.clone();
        let join_handle = thread::spawn(move || {
            thread::sleep(duration);
            // Spawn the new thread
           if let Some(waker) = cloned_waker.lock().unwrap().take() {
                waker.wake();
           }
        });
        SleepContext { shared_waker, waiting_thread: join_handle }
    }
}

#[cfg(test)]
mod tests {
    use super::SleepFuture;
    use std::time::{Duration, Instant};

    #[test]
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
}
