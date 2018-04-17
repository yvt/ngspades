//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ysr2_common;

use ysr2_common::values::DynamicValue;

#[test]
fn new() {
    let x = DynamicValue::new(1919f64);
    assert_eq!(x.get(), 1919f64);
}

#[test]
fn set_slow() {
    let mut x = DynamicValue::new(1f64);
    x.update();
    assert_eq!(x.get(), 1f64);

    x.set_slow(5f64, 4f64);
    assert_eq!(x.get(), 1f64);

    for _ in 0..4 {
        x.update();
    }
    assert_eq!(x.get(), 5f64);
}

#[test]
fn set() {
    let mut x = DynamicValue::new(1f64);
    x.set_slow(5f64, 4f64);
    x.set(3f64);
    x.update();
    assert_eq!(x.get(), 3f64);
}

#[test]
fn next_cusp_time() {
    let mut x = DynamicValue::new(1f64);
    assert_eq!(x.next_cusp_time(32), 32);

    x.set(10f64);
    assert_eq!(x.next_cusp_time(32), 32);

    x.set_slow(20f64, 0.5f64);
    assert_eq!(x.next_cusp_time(32), 1);

    x.set_slow(20f64, 1f64);
    assert_eq!(x.next_cusp_time(32), 1);

    x.set_slow(20f64, 4.5f64);
    assert_eq!(x.next_cusp_time(32), 5);

    x.set_slow(20f64, 5f64);
    assert_eq!(x.next_cusp_time(32), 5);
}
