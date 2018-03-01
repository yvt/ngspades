//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::TestDriver;

pub fn heap_create<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let memory_types = device.caps().memory_types();
        println!(
            "- {} memory types are defined by the device.",
            memory_types.len()
        );

        for (i, memory_type) in memory_types.iter().enumerate() {
            println!("- Creating a heap for [{}] : {:?}", i, memory_type);
            device
                .build_heap()
                .size(256)
                .memory_type(i as _)
                .build()
                .unwrap();
        }
    });
}

pub fn heap_create_fail_zero_size<T: TestDriver>(driver: T) {
    if !driver.is_safe() {
        panic!("this test was skipped because the backend is unsafe");
    }
    driver.for_each_device(&mut |device| {
        device.build_heap().memory_type(0).build().unwrap();
    });
}

pub fn heap_create_fail_missing_memory_type<T: TestDriver>(driver: T) {
    if !driver.is_safe() {
        panic!("this test was skipped because the backend is unsafe");
    }
    driver.for_each_device(&mut |device| {
        device.build_heap().build().unwrap();
    });
}
