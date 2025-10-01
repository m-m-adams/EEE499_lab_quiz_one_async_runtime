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

You are provided with a partial implementation of a custom future, `SleepFuture`, in `src/timer.rs`. 
Your task is to complete `SleepFuture`
`SleepFuture` uses a state machine with the following pattern:
1. **`Created(Duration)`**: Initial state, holds the sleep duration
2. **`Running(JoinHandle, Arc<Mutex<SleepContext>>)`**: Timer thread is active
3. **`Done`**: Timer has completed

### Objectives

- Implement the `spawn_timer_thread` method.
- Implement the `poll` method for the `Future` trait on `SleepFuture`.

### Details

#### 1. `spawn_timer_thread`

- This method should create a thread that completes after a specified duration.
- You will need to set up shared state using `Arc<Mutex<SleepContext>>`.
- Spawn a thread that sleeps for the given duration, then updates the shared state and wakes the future.

#### 2. `Future for SleepFuture: poll`

- If the state is `Created`, call `spawn_timer_thread` to start the timer thread and transition to the `Running` state.
- If the state is `Running`, `poll` method should check if the timer has completed.
  - If completed, transition to `Done` and return ready
  - If not, store the current task's waker in the shared state and return pending.
- If the state is `Done`, return ready.

#### 3. `SleepFuture: drop`
- The timer thread needs its waker removed to close its channel and drop the arc to the executor.

#### Hints

- Use `std::thread::spawn` for the timer thread. It returns a `JoinHandle`
that can be checked for completion with `.is_complete()`
- Use `Arc<Mutex<SleepContext>>` for shared state with the thread (e.g. give it the current waker).
- The waker can be obtained from the context (`cx.waker().clone()`).
- The shared state must be locked before accessing or modifying it.
- `std::thread::park_timeout` will pause the current thread until the specified time or its unparked. 
This allows you to manually unpark it through its handle to wake the thread on drop

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
## Part 3 - Select (select.rs)

### Overview
You are provided with a partial implementation of a select combinator in src/select.rs. Your task is to complete the poll method for the Future trait on the Select struct. This combinator should poll two futures and return the output of the first one that completes, wrapped in the Either enum.

### Objectives
Implement the poll method for Select<A, B>.
Correctly poll both futures and return as soon as one is ready.
Wrap the result in Either::A or Either::B depending on which future completes first.
#### 1. Complete `fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>`
   - Poll both `fut_one` and `fut_two` using Pin::new.
   - If fut_one is ready, return `Poll::Ready(Either::A(output))`.
   - If fut_two is ready, return `Poll::Ready(Either::B(output))`.
   - If neither is ready, return `Poll::Pending`.

### Hints
   - Use `Pin::new(&mut self.fut_one)` and `Pin::new(&mut self.fut_two)` to create a pollable future.
   - The order in which you poll the futures can affect which one wins if both are ready at the same time.
   - You do not need to use unsafe code; the SimpleFuture trait ensures the futures are Unpin.
   - Make sure to poll both futures every time poll is called, we don't track which one is ready
   - If your future from part 1 doesn't implement drop correctly the select tests will fail

# Testing

Unit tests are provided in all three modules. There are additional implementation tests under `tests` that will only work if all three components are implemented successfully.

Run tests with cargo test or the IDE

Passing tests indicate correct implementation. 
The tests in runtime.rs depend on timer.rs being implemented correctly as well!

# Submission

Ensure your code is well-commented and passes all tests before submitting. 
It must also show no errors in `cargo fmt`, `cargo check`, and `cargo clippy`.
Also run the main function to make sure it works. It should print `hello`, sleep 2 seconds, then print `there`