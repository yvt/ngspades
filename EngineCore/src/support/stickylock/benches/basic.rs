//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(test)]
extern crate parking_lot;
extern crate stickylock;
extern crate test;

use std::sync::Mutex;
use stickylock::*;

const N: usize = 1000;

#[bench]
fn std_mutex(b: &mut test::Bencher) {
    let m = Mutex::new(0);
    b.iter(|| {
        for _ in 0..N {
            let _ = m.lock().unwrap();
        }
    });
}

#[bench]
fn parking_lot_mutex(b: &mut test::Bencher) {
    let m = parking_lot::Mutex::new(0);
    b.iter(|| {
        for _ in 0..N {
            let _ = m.lock();
        }
    });
}

#[bench]
fn sticky_mutex(b: &mut test::Bencher) {
    let m = StickyMutex::new(0);
    b.iter(|| {
        for _ in 0..N {
            m.lock();
        }
    });
}

#[bench]
fn sticky_mutex_stick(b: &mut test::Bencher) {
    let m = StickyMutex::new(0);
    m.stick();
    b.iter(|| {
        for _ in 0..N {
            m.lock();
        }
    });
}
