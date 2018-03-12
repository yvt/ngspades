//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(test)]
extern crate xdispatch;
extern crate test;

use test::Bencher;
use xdispatch::*;

#[bench] fn queueing_serial(b: &mut Bencher) {
    let queue = Queue::create("xdispatch test", QueueAttribute::Serial);
    b.iter(move || {
        queue.apply(10000, |_| {});
    });
}

#[bench] fn queueing_concurrent(b: &mut Bencher) {
    let queue = Queue::create("xdispatch test", QueueAttribute::Concurrent);
    b.iter(move || {
        queue.apply(10000, |_| {});
    });
}
