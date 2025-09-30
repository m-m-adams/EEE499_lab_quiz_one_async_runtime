use std::pin::Pin;
use std::task::{Context, Poll};

// just for ease of implementation - otherwise must use unsafe code for the pins
pub trait SimpleFuture: Future + Unpin {}
impl<T: Future + Unpin> SimpleFuture for T {}
pub struct Select<A: SimpleFuture, B: SimpleFuture> {
    fut_one: A,
    fut_two: B,
}

pub enum Either<A, B> {
    A(A),
    B(B),
}
impl<A: SimpleFuture, B: SimpleFuture> Future for Select<A, B> {
    type Output = Either<A::Output, B::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        todo!("poll both futures and return the first one that completes");
    }
}

pub fn select<A: SimpleFuture, B: SimpleFuture>(fut_one: A, fut_two: B) -> Select<A, B> {
    Select { fut_one, fut_two }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timer::SleepFuture;
    use ntest::timeout;
    #[test]
    #[timeout(3000)]
    fn test_select() {
        let f1 = SleepFuture::new(std::time::Duration::new(1, 0));
        let f2 = SleepFuture::new(std::time::Duration::new(2, 0));
        let select = select(f1, f2);
        let start = std::time::Instant::now();
        futures::executor::block_on(select);
        let rt = start.elapsed();
        assert!(rt >= std::time::Duration::new(1, 0));
        assert!(rt <= std::time::Duration::new(2, 0));
    }
}
