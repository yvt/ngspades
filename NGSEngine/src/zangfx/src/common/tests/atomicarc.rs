//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate zangfx_common;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use zangfx_common::{AtomicArc, BArc};

#[test]
fn arc_into_inner_some() {
    let aa = AtomicArc::new(Some(Arc::new(1)));
    assert_eq!(*aa.into_inner().unwrap(), 1);
}

#[test]
fn arc_into_inner_none() {
    let aa: AtomicArc<Arc<u32>> = AtomicArc::empty();
    assert!(aa.into_inner().is_none());
}

#[test]
fn arc_as_ref_some() {
    let mut aa = AtomicArc::new(Some(Arc::new(1)));
    assert_eq!(*aa.as_ref().unwrap(), 1);
}

#[test]
fn arc_as_ref_none() {
    let mut aa: AtomicArc<Arc<u32>> = AtomicArc::empty();
    assert!(aa.as_ref().is_none());
}

#[test]
fn arc_load_some() {
    let mut aa = AtomicArc::new(Some(Arc::new(1)));
    assert_eq!(*aa.load().unwrap(), 1);
}

#[test]
fn arc_load_none() {
    let mut aa: AtomicArc<Arc<u32>> = AtomicArc::empty();
    assert!(aa.load().is_none());
}

#[test]
fn arc_swap() {
    let aa = AtomicArc::new(Some(Arc::new(1)));
    let old = aa.swap(Some(Arc::new(2)), Ordering::Relaxed);
    assert_eq!(*old.unwrap(), 1);
    assert_eq!(*aa.into_inner().unwrap(), 2);
}

#[test]
fn arc_compare_and_swap1() {
    let cur = Some(Arc::new(1));
    let aa = AtomicArc::new(cur.clone());
    let old = aa.compare_and_swap(&cur, Some(Arc::new(2)), Ordering::Relaxed);
    assert_eq!(*old.unwrap().unwrap(), 1);
    assert_eq!(*aa.into_inner().unwrap(), 2);
}

#[test]
fn arc_compare_and_swap2() {
    let cur = Some(Arc::new(114514));
    let aa = AtomicArc::new(Some(Arc::new(1)));
    let old = aa.compare_and_swap(&cur, Some(Arc::new(2)), Ordering::Relaxed);
    assert_eq!(*old.unwrap_err().unwrap(), 2);
    assert_eq!(*aa.into_inner().unwrap(), 1);
}

// -----------------


#[test]
fn barc_into_inner_some() {
    let aa = AtomicArc::new(Some(BArc::new(1)));
    assert_eq!(*aa.into_inner().unwrap(), 1);
}

#[test]
fn barc_into_inner_none() {
    let aa: AtomicArc<BArc<u32>> = AtomicArc::empty();
    assert!(aa.into_inner().is_none());
}

#[test]
fn barc_as_ref_some() {
    let mut aa = AtomicArc::new(Some(BArc::new(1)));
    assert_eq!(*aa.as_ref().unwrap(), 1);
}

#[test]
fn barc_as_ref_none() {
    let mut aa: AtomicArc<BArc<u32>> = AtomicArc::empty();
    assert!(aa.as_ref().is_none());
}

#[test]
fn barc_load_some() {
    let mut aa = AtomicArc::new(Some(BArc::new(1)));
    assert_eq!(*aa.load().unwrap(), 1);
}

#[test]
fn barc_load_none() {
    let mut aa: AtomicArc<BArc<u32>> = AtomicArc::empty();
    assert!(aa.load().is_none());
}

#[test]
fn barc_swap() {
    let aa = AtomicArc::new(Some(BArc::new(1)));
    let old = aa.swap(Some(BArc::new(2)), Ordering::Relaxed);
    assert_eq!(*old.unwrap(), 1);
    assert_eq!(*aa.into_inner().unwrap(), 2);
}

#[test]
fn barc_compare_and_swap1() {
    let cur = Some(BArc::new(1));
    let aa = AtomicArc::new(cur.clone());
    let old = aa.compare_and_swap(&cur, Some(BArc::new(2)), Ordering::Relaxed);
    assert_eq!(*old.unwrap().unwrap(), 1);
    assert_eq!(*aa.into_inner().unwrap(), 2);
}

#[test]
fn barc_compare_and_swap2() {
    let cur = Some(BArc::new(114514));
    let aa = AtomicArc::new(Some(BArc::new(1)));
    let old = aa.compare_and_swap(&cur, Some(BArc::new(2)), Ordering::Relaxed);
    assert_eq!(*old.unwrap_err().unwrap(), 2);
    assert_eq!(*aa.into_inner().unwrap(), 1);
}
