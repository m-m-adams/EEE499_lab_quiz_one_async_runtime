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
use thiserror::Error;
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

/// Task executor that receives tasks off of a channel and runs them. The receiver end of the
/// channel is stored here, and a clone of the sender end is stored in each `Task`.
pub struct Runtime {
    ready_queue: Receiver<Arc<Task>>,
    task_sender: Option<SyncSender<Arc<Task>>>,
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Task channel closed")]
    TaskChannelClosed,
    #[error("Task channel full")]
    TaskChannelFull(#[from] std::sync::mpsc::TrySendError<Arc<Task>>),
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

impl Runtime {
    pub fn new() -> Self {
        const MAX_QUEUED_TASKS: usize = 10_000;
        let (task_sender, ready_queue) = sync_channel(MAX_QUEUED_TASKS);
        Runtime {
            ready_queue,
            task_sender: Some(task_sender),
        }
    }
    /// Spawn a new future onto the task channel. The future should be any type which implements
    /// `Future` with an output type of `()` that can be sent between threads and have a static lifetime
    pub fn spawn(
        &self,
        future: impl Future<Output = ()> + 'static + Send,
    ) -> Result<(), RuntimeError> {
        match self.task_sender.as_ref() {
            Some(sender) => {
                // box the future - we want to be able to store different types of futures
                // in the same data structure, and boxing it gives it a consistent size
                let future = future.boxed();
                let task = Arc::new(Task {
                    future: Mutex::new(Some(future)),
                    task_sender: sender.clone(),
                });
                sender.try_send(task)?;
                Ok(())
            }
            None => Err(RuntimeError::TaskChannelClosed),
        }
    }

    pub fn run(mut self) {
        self.task_sender = None;
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

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }   
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer::SleepFuture;
    async fn increment_count(count: Arc<Mutex<u32>>) {
        SleepFuture::new(std::time::Duration::new(0, 100)).await;
        let mut num = count.lock().unwrap();
        *num += 1;
    }
    async fn increment_count_twice(count: Arc<Mutex<u32>>) {
        increment_count(count.clone()).await;
        increment_count(count).await;
    }
    #[test]
    fn test_run_all_tasks() {
        let count = Arc::new(Mutex::new(0));
        let executor = Runtime::new();
        // Spawn tasks to wait and then increment the count
        for _ in 0..10 {
            let count_clone = count.clone();
            executor
                .spawn(increment_count(count_clone))
                .expect("failed to spawn");
        }

        executor.run();
        // check that the count was incremented by 10 and the executor finished
        assert_eq!(*count.lock().unwrap(), 10);
    }
    #[test]
    fn test_parallel() {
        let executor = Runtime::new();
        for _ in 0..10 {
            executor
                .spawn(SleepFuture::new(std::time::Duration::new(1, 0)))
                .expect("TODO: panic message");
        }
        let start = std::time::Instant::now();
        executor.run();
        // All 10 timers should complete in just over 1 second, give 10ms grace period for windows thread overhead
        let rt = start.elapsed();
        let target = std::time::Duration::new(1, 10_000_000);
        println!("Elapsed: {:?}, Target: {:?}", rt, target);
        assert!(rt < target);
    }

    #[test]
    fn test_series() {
        let count = Arc::new(Mutex::new(0));
        let executor = Runtime::new();
        executor
            .spawn(increment_count_twice(count.clone()))
            .expect("failed to spawn");

        executor.run();
        // check that the count was incremented by 10 and the executor finished
        assert_eq!(*count.lock().unwrap(), 2);
    }
}
