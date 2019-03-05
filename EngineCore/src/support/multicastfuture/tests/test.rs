#![feature(futures_api)]
use futures::{executor::block_on, future::lazy, prelude::*};
use multicastfuture::MultiCast;
use std::{marker::Unpin, pin::Pin};

#[test]
fn consumers_one() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    assert_eq!(block_on(con1), 42);
}

#[test]
fn consumers_two_1() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    let con2 = Pin::new(&mc).subscribe();
    assert_eq!(block_on(con1), 42);
    assert_eq!(block_on(con2), 42);
}

/* Will block indefinitely
#[test]
fn consumers_two_2() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    let con2 = Pin::new(&mc).subscribe();
    assert_eq!(block_on(con2), 42);
    assert_eq!(block_on(con1), 42);
}
*/

#[test]
fn consumers_two_3() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    let con2 = Pin::new(&mc).subscribe();
    assert_eq!(block_on(con1.join(con2)), (42, 42));
}

#[test]
fn consumers_two_4() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    let con2 = Pin::new(&mc).subscribe();
    assert_eq!(block_on(con2.join(con1)), (42, 42));
}

#[test]
fn consumers_three() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    let con2 = Pin::new(&mc).subscribe();
    let con3 = Pin::new(&mc).subscribe();
    assert_eq!(block_on(con1), 42);
    assert_eq!(block_on(con2), 42);
    assert_eq!(block_on(con3), 42);
}

#[test]
fn delete_leader() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    let con2 = Pin::new(&mc).subscribe();
    drop(con1);
    assert_eq!(block_on(con2), 42);
}

#[test]
fn delete_nonleader() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    let con2 = Pin::new(&mc).subscribe();
    drop(con2);
    assert_eq!(block_on(con1), 42);
}

#[test]
fn delete_all() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    let con2 = Pin::new(&mc).subscribe();
    drop(con1);
    drop(con2);
    let con3 = Pin::new(&mc).subscribe();
    assert_eq!(block_on(con3), 42);
}

#[test]
fn already_has_result() {
    let mc = MultiCast::new(lazy(|_| 42));
    let con1 = Pin::new(&mc).subscribe();
    assert_eq!(block_on(con1), 42);
    let con2 = Pin::new(&mc).subscribe();
    assert_eq!(block_on(con2), 42);
}

#[test]
fn unsize() {
    let mc = MultiCast::new(lazy(|_| 42u32));
    let mc: &MultiCast<dyn Future<Output = u32> + Unpin> = &mc;
    let con1 = Pin::new(mc).subscribe();
    assert_eq!(block_on(con1), 42);
}
