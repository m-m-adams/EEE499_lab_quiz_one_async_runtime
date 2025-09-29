# Async Lab Quiz

This lab quiz will guide you through building core components of an async runtime in Rust. 
You will implement a custom future (TimerFuture) and a simple executor to run asynchronous tasks. 
The quiz is split into two parts, the first has you build a future for a timer and the second has you build a general purpose executer. 
By completing this quiz, you will gain hands-on experience with Rust's concurrency primitives and async infrastructure. 
All work should be completed in the provided files, and your solutions will be validated by unit tests.
When you are finished make sure you can run the sample main function.

Do not modify the tests or import any more libraries

## Part 1 - TimerFuture (timer.rs)

### Overview

You are provided with a partial implementation of a custom future, `TimerFuture`, in `src/timer.rs`. Your task is to complete the `new` constructor and the `poll` method for `TimerFuture`.

### Objectives

- Implement the `TimerFuture::new(duration: Duration) -> Self` method.
- Implement the `poll` method for the `Future` trait on `TimerFuture`.

### Details

#### 1. `TimerFuture::new`

- This method should create a new `TimerFuture` that completes after a specified duration.
- You will need to set up shared state using `Arc<Mutex<SharedState>>`.
- Spawn a thread that sleeps for the given duration, then updates the shared state and wakes the future.

#### 2. `Future for TimerFuture: poll`

- The `poll` method should check if the timer has completed.
- If completed, return ready
- If not, store the current task's waker in the shared state and return pending.

#### Hints

- Use `std::thread::spawn` for the timer thread.
- Use `std::sync::{Arc, Mutex}` for shared state.
- The waker can be obtained from the context (`cx.waker().clone()`).
- The shared state should be locked before accessing or modifying it.

## Part 2 - Runtime (runtime.rs)

### Overview

You are provided with a partial implementation of a simple async runtime in `src/runtime.rs`. Your task is to complete the following methods:

- `Task::wake_by_ref`
- `Spawner::spawn`
- `Executor::run`

### Objectives

#### 1. `Task::wake_by_ref`

- Implement the `ArcWake` trait for `Task`.
- When a task is woken, it should be sent back onto the task queue so it can be polled again.
- `SyncSender` has a TrySend function which accepts an `Arc<Task>` (which is also `Arc<Self>` for a task)

#### 2. `Spawner::spawn`

- Implement the method to spawn new futures onto the task queue.
- Box the future, wrap it in a `Task`, and send it to the queue.

#### 3. `Executor::run`

- Implement the main loop that receives tasks from the queue and polls them.
- If a future is not complete, put it back in the queue to be polled again.

### Hints

- The provided types in the partial implementation walk you through the steps
- Use `Arc` for shared ownership of tasks.
- Use `Mutex` to safely access the future inside a task. Call `.lock().unwrap()` to lock the mutex
(blocks until it's ready, don't use this for real in embassy)
- Use the provided channel for task scheduling.
- Use `waker_ref(&task)` to create a waker from a task.
- `.take()` can be used on options to return the inner value and set the option to none. 
This is good for taking ownership of the future inside a task.
- `if let` can be used to determine whether a type matches a pattern.
- This example shows how both work
``` 
    if let Some(mut optional_value) = optional.take() {
       println!("got a value: {}", optional_value);
    } 
   
   ```

# Testing

Unit tests are provided in both modules. Run them with cargo test or the IDE

Passing tests indicate correct implementation. 
The tests in runtime.rs depend on timer.rs being implemented correctly as well!

# Submission

Ensure your code is well-commented and passes all tests before submitting.

Also run the main function to make sure it works. It should print `hello`, sleep 2 seconds, then print `there`