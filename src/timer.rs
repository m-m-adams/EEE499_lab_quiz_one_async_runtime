use std::thread::JoinHandle;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::{Duration, Instant},
};
use std::cmp::Reverse;
use std::hash::Hash;
use std::sync::LazyLock;
use std::sync::mpsc::{channel, Receiver, Sender};
use priority_queue::PriorityQueue;
use rand::random;

#[derive(Debug)]
struct SleepContext {
    shared_waker: Option<Waker>,
    done: bool
}

#[derive(Debug)]
struct SleepMessage {
    waker: Arc<Mutex<SleepContext>>,
    duration: Duration,
}

#[derive(Debug)]
struct SleepItem {
    waker: Arc<Mutex<SleepContext>>,
    wake_time: Instant,
    rand_id: u64,
}

impl Hash for SleepItem {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.wake_time.hash(state);
        self.rand_id.hash(state);
    }
}

impl PartialEq<Self> for SleepItem {
    fn eq(&self, other: &Self) -> bool {
        self.wake_time == other.wake_time && self.rand_id == other.rand_id
    }
}

impl Eq for SleepItem {}


impl SleepItem {
    fn new(msg: SleepMessage) -> Self {
        SleepItem {
            waker: msg.waker,
            wake_time: Instant::now() + msg.duration,
            rand_id: random::<u64>()
        }
    }
    fn time_left(&self) -> Duration {
        self.wake_time.duration_since(Instant::now())
    }

    fn is_expired(&self) -> bool {
        self.time_left() <= Duration::from_millis(0)
    }

    fn wake(self) {
        // time to wake up the associated future
        let mut context = self.waker.lock().unwrap();
        let mut shared_waker = context.shared_waker.take();
        if let Some(waker) = shared_waker.take() {
            dbg!("calling wake");

            waker.wake();
            context.done = true;
        }
    }
}

static SLEEP_ITEM: Mutex<Option<SleepItem>> = Mutex::new(None);

fn set_item_and_wake(item: SleepItem, handle: &JoinHandle<()>) {
    dbg!("setting item", &item);
    *SLEEP_ITEM.lock().unwrap() = Some(item);
    handle.thread().unpark();
}
fn sleep_until_wake_inner() -> ! {
    loop {
        let time: Option<Duration> = {
            let mut g = SLEEP_ITEM.lock().unwrap();
            match g.take() {
                None => None,
                Some(item) => {
                    dbg!("checking item", &item);
                    if item.is_expired() {
                        item.wake();
                        None
                    } else {
                        // not woken up yet, put it back and park until it's time
                        let sleep_time = item.time_left();
                        dbg!("sleeping for {:?} at {:?}", sleep_time, Instant::now());
                        *g = Some(item);
                        Some(sleep_time)
                    }
                }
            }
        };
        if let Some(time) = time {
            dbg!("parking for {time:?}");
            {
                let mut g = SLEEP_ITEM.lock().unwrap();
                let item = g.as_ref().unwrap();
                dbg!("checking item", &g);

            }
            thread::park_timeout(time);
        } else {
            dbg!("parking indefinitely");

            thread::park();
        }

    }
}

fn sleep_until_wake() -> JoinHandle<()> {
    thread::spawn(move || {
        sleep_until_wake_inner();
    })
}

fn sleep_message_reciever(rx: Receiver<SleepMessage>, handle: JoinHandle<()>) {
    let mut sleep_queue: PriorityQueue<SleepItem, Reverse<Instant>>= PriorityQueue::new();
    dbg!("waiting for items",);

    for msg in rx {
        dbg!(&msg);

        let item = SleepItem::new(msg);
        let time =item.wake_time;
        sleep_queue.push(item, Reverse(time));
        if let Some(current)= SLEEP_ITEM.lock().unwrap().take() {
            if let Some(highest) = {sleep_queue.pop()} {
                if highest.0.wake_time < Instant::now() {
                    dbg!("wake time in the past");
                    highest.0.wake();
                }
                else if highest.0.wake_time < current.wake_time {
                    let item = sleep_queue.pop().unwrap().0;
                    set_item_and_wake(item, &handle);
                    let p = Reverse(current.wake_time);
                    sleep_queue.push(current, p);
                } else {
                    sleep_queue.push(highest.0, highest.1);
                }
            }
        } else {
            dbg!("no current item");
            set_item_and_wake(sleep_queue.pop().unwrap().0, &handle);
        }
    }
}

static SLEEP_QUEUE: LazyLock<Sender<SleepMessage>> = LazyLock::new (|| {
    dbg!("init sleep queue",);

    let (tx, rx) = channel::<SleepMessage>();
    let h = sleep_until_wake();
    std::thread::spawn(move ||  {
        sleep_message_reciever(rx, h);
    });
    tx
});


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



impl SleepFuture {
    /// Create a new `SleepFuture` which will complete after a timeout
    pub fn new(duration: Duration) -> Self {
        SleepFuture {
            state: SleepState::Created(duration),
        }
    }

    fn spawn_timer_thread(mut self: Pin<&mut Self>, cx: &mut Context, duration: Duration) {
        dbg!("Spawning timer thread for {:?}", duration);
        let context = SleepContext {
            shared_waker: Some(cx.waker().clone()),
            done: false
        };
        let context = Arc::new(Mutex::new(context));
        let cloned_ctx = context.clone();
        let msg = SleepMessage { waker: cloned_ctx, duration };
        SLEEP_QUEUE.send(msg).unwrap();
        self.state = SleepState::Running(context);
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
            SleepState::Running(ref ctx) => {
                if ctx.lock().unwrap().done {
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
        if let SleepState::Running(context) = &self.state {
            let mut ctx = context.lock().unwrap();
            ctx.shared_waker = None;
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
