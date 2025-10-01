use std::thread::JoinHandle;
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
    Running(JoinHandle<()>, Arc<Mutex<SleepContext>>),
    /// the future has completed
    Done,
}

struct SleepContext {
    shared_waker: Option<Waker>,
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
        };
        let context = Arc::new(Mutex::new(context));
        let cloned_ctx = context.clone();
        let h = thread::spawn(move || {
            thread::park_timeout(duration);
            let mut c = cloned_ctx.lock().unwrap();
            // Spawn the new thread
            if let Some(waker) = c.shared_waker.take() {
                waker.wake();
            }
        });
        self.state = SleepState::Running(h, context);
    }
}

impl Future for SleepFuture {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            SleepState::Created(duration) => {
                self.spawn_timer_thread(cx, duration);
                Poll::Pending
            }
            SleepState::Running(ref handle, ref ctx) => {
                if handle.is_finished() {
                    self.state = SleepState::Done;
                    Poll::Ready(())
                } else {
                    let mut ctx = ctx.lock().unwrap();
                    ctx.shared_waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
            SleepState::Done => Poll::Ready(()),
        }
    }
}

impl Drop for SleepFuture {
    fn drop(&mut self) {
        if let SleepState::Running(handle, context) = &self.state {
            let mut ctx = context.lock().unwrap();
            ctx.shared_waker = None;
            handle.thread().unpark(); // wake up the thread so it ends now
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
    #[timeout(3000)]
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
    #[timeout(3000)]
    fn test_timers_start() {
        let start = Instant::now();
        let timer = SleepFuture::new(Duration::from_millis(200));
        thread::sleep(Duration::from_millis(100));
        futures::executor::block_on(timer);
        let run_time = start.elapsed();
        assert!(run_time >= Duration::from_millis(300));
    }
}
