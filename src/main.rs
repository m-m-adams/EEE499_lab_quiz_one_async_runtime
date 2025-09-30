use eee499_timer::runtime::*;
use eee499_timer::timer::*;
use std::io;
use std::io::Write;
use std::time::Duration;

async fn hello_there() {
    print!("hello");
    io::stdout().flush().unwrap();
    // Wait for our timer future to complete after two seconds.
    SleepFuture::new(Duration::new(2, 0)).await;
    println!(" there!");
}
fn main() {
    let executor = Runtime::new();

    // Spawn a task to print before and after waiting on a timer.
    executor.spawn(hello_there()).expect("failed to spawn");

    // Run the executor until the task queue is empty.
    // This will print "howdy!", pause, and then print "done!".
    executor.run();
}
