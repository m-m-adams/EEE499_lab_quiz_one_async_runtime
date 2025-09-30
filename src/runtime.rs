use futures::{
    future::{BoxFuture, FutureExt},
    task::{ArcWake, waker_ref},
};
use std::{
    future::Future,
    sync::mpsc::{Receiver, SyncSender, sync_channel},
    sync::{Arc, Mutex},
    task::Context,
};

/// A future that can reschedule itself to be polled by an `Executor`.
pub struct Task {
    /// In-progress future that should be pushed to completion.
    ///
    /// The `Mutex` is not necessary for correctness, since we only have
    /// one thread executing tasks at once. However, Rust isn't smart
    /// enough to know that `future` is only mutated from one thread,
    /// so we need to use the `Mutex` to prove thread-safety. A production
    /// executor would not need this, and could use `UnsafeCell` instead.
    future: Mutex<Option<BoxFuture<'static, ()>>>,

    /// Handle to place the task itself back onto the task queue when ready
    task_sender: SyncSender<Arc<Task>>,
}

/// The arcwake trait will allow us to turn an Arc<Task> into a Waker
impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // Implement `wake` by sending this task back onto the task channel
        // so that it will be polled again by the executor.
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .try_send(cloned)
            .expect("too many tasks queued");
    }
}

/// Task executor that receives tasks off of a channel and runs them. The receiver end of the
/// channel is stored here, and a clone of the sender end is stored in each `Task`.
pub struct Executor {
    ready_queue: Receiver<Arc<Task>>,
}

/// `Spawner` spawns new futures onto the task channel.
#[derive(Clone)]
pub struct Spawner {
    task_sender: SyncSender<Arc<Task>>,
}

impl Spawner {
    /// Spawn a new future onto the task channel. The future should be any type which implements
    /// `Future` with an output type of `()` that can be sent between threads and have a static lifetime
    pub(crate) fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        // box the future - we want to be able to store different types of futures
        // in the same data structure, and boxing it gives it a consistent size
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });
        self.task_sender
            .try_send(task)
            .expect("too many tasks queued");
    }
}

impl Executor {
    pub(crate) fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() {
            // Take the future, and if it has not yet completed (is still Some),
            // poll it in an attempt to complete it.
            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                // Create a `WakerRef` from the task itself
                let waker = waker_ref(&task);

                let context = &mut Context::from_waker(&waker);
                // `BoxFuture<T>` is a type alias for
                // `Pin<Box<dyn Future<Output = T> + Send + 'static>>`.
                // We can get a `Pin<&mut dyn Future + Send + 'static>`
                // from it by calling the `Pin::as_mut` method.
                if future.as_mut().poll(context).is_pending() {
                    // We're not done processing the future, so put it
                    // back in its task to be run again in the future.
                    *future_slot = Some(future);
                }
            }
        }
    }
}

pub fn new_executor_and_spawner() -> (Executor, Spawner) {
    // Maximum number of tasks to allow queueing in the channel at once.
    // This is just to make `sync_channel` happy, and wouldn't be present in
    // a real executor.
    const MAX_QUEUED_TASKS: usize = 10_000;
    let (task_sender, ready_queue) = sync_channel(MAX_QUEUED_TASKS);
    (Executor { ready_queue }, Spawner { task_sender })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer::TimerFuture;
    async fn increment_count(count: Arc<Mutex<u32>>) {
        TimerFuture::new(std::time::Duration::new(0, 100)).await;
        let mut num = count.lock().unwrap();
        *num += 1;
    }
    #[test]
    fn test_run_all_tasks() {
        let count = Arc::new(Mutex::new(0));
        let (executor, spawner) = new_executor_and_spawner();
        // Spawn a task to print before and after waiting on a timer.
        for _ in 0..10 {
            let count_clone = count.clone();
            spawner.spawn(increment_count(count_clone));
        }
        // Drop the spawner to drop its reference to the task sending channel
        // otherwise the runtime will be stuck waiting for new tasks
        drop(spawner);
        // Run the executor until the task queue is empty.
        executor.run();
        assert_eq!(*count.lock().unwrap(), 10);
    }
    #[test]
    fn test_parallel() {
        let (executor, spawner) = new_executor_and_spawner();
        for _ in 0..10 {
            spawner.spawn(TimerFuture::new(std::time::Duration::new(1, 0)));
        }
        drop(spawner);
        let start = std::time::Instant::now();
        executor.run();
        // All 10 timers should complete in just over 1 second, give 10ms grace period for windows thread overhead
        let rt = start.elapsed();
        let target = std::time::Duration::new(1, 10_000_000);
        println!("Elapsed: {:?}, Target: {:?}", rt, target);
        assert!(rt < target);
    }
}
