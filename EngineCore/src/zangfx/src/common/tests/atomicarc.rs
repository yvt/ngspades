//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate zangfx_common;
extern crate atom2;
use std::sync::atomic::Ordering;
use atom2::Atom;
use zangfx_common::BArc;

#[test]
fn barc_into_inner_some() {
    let aa = Atom::new(Some(BArc::new(1)));
    assert_eq!(*aa.into_inner().unwrap(), 1);
}

#[test]
fn barc_into_inner_none() {
    let aa: Atom<BArc<u32>> = Atom::empty();
    assert!(aa.into_inner().is_none());
}

#[test]
fn barc_as_ref_some() {
    let mut aa = Atom::new(Some(BArc::new(1)));
    assert_eq!(*aa.as_inner_ref().unwrap(), 1);
}

#[test]
fn barc_as_ref_none() {
    let mut aa: Atom<BArc<u32>> = Atom::empty();
    assert!(aa.as_inner_ref().is_none());
}

#[test]
fn barc_load_some() {
    let mut aa = Atom::new(Some(BArc::new(1)));
    assert_eq!(*aa.load().unwrap(), 1);
}

#[test]
fn barc_load_none() {
    let mut aa: Atom<BArc<u32>> = Atom::empty();
    assert!(aa.load().is_none());
}

#[test]
fn barc_swap() {
    let aa = Atom::new(Some(BArc::new(1)));
    let old = aa.swap(Some(BArc::new(2)), Ordering::Relaxed);
    assert_eq!(*old.unwrap(), 1);
    assert_eq!(*aa.into_inner().unwrap(), 2);
}

#[test]
fn barc_compare_and_swap1() {
    let cur = Some(BArc::new(1));
    let aa = Atom::new(cur.clone());
    let old = aa.compare_and_swap(&cur, Some(BArc::new(2)), Ordering::Relaxed);
    assert_eq!(*old.unwrap().unwrap(), 1);
    assert_eq!(*aa.into_inner().unwrap(), 2);
}

#[test]
fn barc_compare_and_swap2() {
    let cur = Some(BArc::new(114514));
    let aa = Atom::new(Some(BArc::new(1)));
    let old = aa.compare_and_swap(&cur, Some(BArc::new(2)), Ordering::Relaxed);
    assert_eq!(*old.unwrap_err().unwrap(), 2);
    assert_eq!(*aa.into_inner().unwrap(), 1);
}
