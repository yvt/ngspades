//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![feature(test)]
extern crate test;
#[macro_use]
extern crate objc;

use objc::runtime::{Class, Object};

#[bench]
fn msg_send_1000(b: &mut test::Bencher) {
    let cls = Class::get("NSObject").unwrap();
    let obj: *mut Object = unsafe { msg_send![cls, new] };
    b.iter(|| {
        for _ in 0..1000 {
            let () = unsafe { msg_send![obj, retain] };
            let () = unsafe { msg_send![obj, release] };
        }
    });
    let () = unsafe { msg_send![obj, release] };
}