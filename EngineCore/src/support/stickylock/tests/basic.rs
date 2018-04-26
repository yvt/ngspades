//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate stickylock;

use stickylock::*;

#[test]
fn lock_success() {
    let k = StickyMutex::new(42);
    assert_eq!(*k.lock(), 42);
}

#[test]
fn try_lock_success() {
    let k = StickyMutex::new(42);
    assert_eq!(*k.try_lock().unwrap(), 42);
}

#[test]
#[should_panic]
fn lock_twice_panic() {
    let k = StickyMutex::new(42);
    let _x = k.lock();
    k.lock();
}

#[test]
fn try_lock_twice_fail() {
    let k = StickyMutex::new(42);
    let _x = k.lock();
    assert!(k.try_lock().is_none());
}

#[test]
fn stick() {
    let k = StickyMutex::new(42);
    k.stick();
    k.unstick().unwrap();
}

#[test]
fn unstick_excessive() {
    let k = StickyMutex::new(42);
    k.stick();
    k.unstick().unwrap();
    assert_eq!(k.unstick(), Err(UnstickError::NotLocked));
}

#[test]
fn stick2() {
    let k = StickyMutex::new(42);
    k.stick();
    k.stick();
    k.unstick().unwrap();
    k.unstick().unwrap();
}

#[test]
fn unstick2_excessive() {
    let k = StickyMutex::new(42);
    k.stick();
    k.stick();
    k.unstick().unwrap();
    k.unstick().unwrap();
    assert_eq!(k.unstick(), Err(UnstickError::NotLocked));
}

#[test]
fn unstick_after_unlock() {
    let k = StickyMutex::new(42);
    k.stick();
    k.lock();
    k.unstick().unwrap();
}

#[test]
fn unstick_before_unlock() {
    let k = StickyMutex::new(42);
    k.stick();
    let _x = k.lock();
    k.unstick().unwrap();
}
