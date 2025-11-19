# Async Lab Quiz

This lab quiz will guide you through building core components of an async runtime in Rust. 
You will implement a custom future (SleepFuture) and a simple executor to run asynchronous tasks. 
The quiz is split into two parts, the first has you build a future for a timer and the second has you build a general purpose executer. 
By completing this quiz, you will gain hands-on experience with Rust's concurrency primitives and async infrastructure. 
All work should be completed in the provided files, and your solutions will be validated by unit tests.
When you are finished make sure you can run the sample main function.

Do not modify the tests or import any more libraries

## Part 1 - SleepFuture (timer.rs)

### Overview

You are provided with the implementation of a custom future, `SleepFuture`, in `src/timer.rs`. 
This is the same future as used to complete quiz 1
`SleepFuture` uses a state machine with the following pattern:
1. **`Created(Duration)`**: Initial state, holds the sleep duration
2. **`Running(JoinHandle, Arc<Mutex<SleepContext>>)`**: Timer thread is active
3. **`Done`**: Timer has completed
Your task is to modify it so that it uses only a single background thread to manage all timers.
This refactor has been started for you but you must complete both the background thread and the wake setup function 

### Objectives

- Complete the `setup_delayed_wake` method.
- Implement the `poll` method for the `Future` trait on `SleepFuture`.

### Details

#### 1. `setup_delayed_wake`

- This method should:
  - push the `Arc<Mutex<SleepContext>>` into the global `SLEEP_QUEUE`.
  - set the global `SIGNAL` to true (atomic store) so the background thread wakes and re-checks the queue.

#### 2. Background sleep thread (SLEEP_THREAD closure)

- The background thread runs a loop and the file contains several TODOs. Implementations should:
  - Lock the `SLEEP_QUEUE` (acquire the mutex) to read/update entries.
  - Iterate through the queue and 
    1. call `wake_if_needed()` on each `SleepContext` (the method will set `completed` and wake the stored waker when a timer has expired).
    2. remove the expired entries from the queue
  - Determine the next minimum `end_time` among contexts to decide how long to wait (use `Instant::now() + Duration::from_secs(1)` as a max/default).
  - Use an atomic compare and swap (compare_exchange or similar) to set `SIGNAL` to false only if it is currently true (this clears the "someone enqueued a timer" flag).
  - Spin/wait (hint::spin_loop) while SIGNAL is true and the minimum end time is not yet reached
  
### Hints
- For the atomic compare-and-swap use `SIGNAL.compare_exchange` (or `compare_exchange_weak`) to conditionally set the flag.
- Use `Instant::now()` to get the current time
- Use the vector method `.retain(fn(item) -> bool)` to remove completed contexts from the queue 
- use the vector method `.fold(initial, fn(acc, item) -> new_acc)` to iterate through the queue and find the minimum end time


# Submission

Ensure your code is readable, well-commented and passes all tests before submitting. 
It must also show no errors in `cargo fmt`, `cargo check`, and `cargo clippy`.
Do not modify any code outside of two mentioned functions