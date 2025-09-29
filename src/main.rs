mod runtime;
mod timer;

use runtime::*;
use std::io;
use std::io::Write;
use std::time::Duration;
use timer::*;

async fn hello_there() {
    print!("hello");
    io::stdout().flush().unwrap();
    // Wait for our timer future to complete after two seconds.
    TimerFuture::new(Duration::new(2, 0)).await;
    println!(" there!");
}
fn main() {
    let (executor, spawner) = new_executor_and_spawner();

    // Spawn a task to print before and after waiting on a timer.
    spawner.spawn(hello_there());

    // Drop the spawner to drop its reference to the task sending channel
    // otherwise the runtime will be stuck waiting for new tasks
    drop(spawner);

    // Run the executor until the task queue is empty.
    // This will print "howdy!", pause, and then print "done!".
    executor.run();
}
