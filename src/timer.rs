use std::{future::Future, hint, pin::Pin, sync::{Arc, Mutex}, task::{Context, Poll, Waker}, thread, time::{Duration, Instant}};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{LazyLock, MutexGuard};
use std::thread::JoinHandle;

// this is a mutex around a vector of sleep contexts. The outer mutex synchronizes access to the vector,
// the inner mutexes are used to synchronize access to each individual context
static SLEEP_QUEUE: Mutex<Vec<Arc<Mutex<SleepContext>>>> = Mutex::new(
    Vec::new()
);

// use this atomic signal to tell the sleep thread that there is a new item in the queue
static SIGNAL: AtomicBool = AtomicBool::new(false);

static SLEEP_THREAD: LazyLock<JoinHandle<()>> = LazyLock::new(
    {move || thread::spawn(|| loop {
        // todo Lock the sleep queue
        let mut queue: MutexGuard<Vec<Arc<Mutex<SleepContext>>>> = todo!();

        // todo iterate throug the queue, wake any threads that are expired using the .wake_if_needed() method

        // Find the next expiring timer to figure out when to sleep until. Use 1 second as the max sleep
        let default = Instant::now() + Duration::new(1,0);

        //todo iterate through the queue to find the minimum end_time
        let min: Instant = todo!();

        drop(queue);

        //todo write a compare and swap atomic operation to set the signal to false iff it is true

        while todo!("Compare and swap to check if the signal is true") && Instant::now() < min {
            hint::spin_loop();
        }


    })}
);

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
    end_time: Instant,
}

impl SleepContext {
    // returns true if not done yet
    fn wake_if_needed(&mut self) -> bool {
        if self.end_time < Instant::now()
            && let Some(waker) = self.shared_waker.take() {
                self.completed = true;
                waker.wake();
                return false;
            }
    true
    }
}

impl SleepFuture {
    /// Create a new `SleepFuture` which will complete after a timeout
    pub fn new(duration: Duration) -> Self {
        SleepFuture {
            state: SleepState::Created(duration),
        }
    }

    fn setup_delayed_wake(mut self: Pin<&mut Self>, cx: &mut Context, duration: Duration) {
        if SLEEP_THREAD.is_finished() {
            // should never happen, we just need to access it once to make sure it's started
            panic!("oh no")
        }
        let context = SleepContext {
            shared_waker: Some(cx.waker().clone()),
            completed: false,
            end_time: Instant::now() + duration,
        };
        let context = Arc::new(Mutex::new(context));
        let cloned_ctx = context.clone();
        SLEEP_QUEUE.lock().unwrap().push(cloned_ctx);
        todo!("Set the signal to true using an atomic store operation");
        self.state = SleepState::Running(context);
    }
}

impl Future for SleepFuture {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            SleepState::Created(duration) => {
                self.setup_delayed_wake(cx, duration);
                Poll::Pending
            }
            SleepState::Running(ref ctx) => {
                {
                    let mut ctx = ctx.lock().unwrap();
                    if !ctx.completed {
                        ctx.shared_waker = Some(cx.waker().clone());
                        return Poll::Pending;
                    }
                }
                self.state = SleepState::Done;
                Poll::Ready(())
            }
            SleepState::Done => Poll::Ready(()),
        }
    }
}

impl Drop for SleepFuture {
    fn drop(&mut self) {
        if let SleepState::Running(context) = &self.state {
            let mut ctx = context.lock().unwrap();
            ctx.shared_waker = None;
            ctx.completed = true;
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
