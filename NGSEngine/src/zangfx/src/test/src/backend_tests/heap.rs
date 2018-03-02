//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use gfx;
use common::BinaryInteger;
use super::{utils, TestDriver};

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

pub fn heap_alloc_buffer<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let memory_types = device.caps().memory_types();

        let mut builder = device.build_buffer();
        builder
            .size(1001)
            .usage(flags![gfx::BufferUsage::{CopyRead | CopyWrite}]);

        println!("- Creating a buffer");
        let mut buffer = utils::UniqueBuffer::new(device, builder.build().unwrap());

        println!("- Querying the memory requirement for the buffer");
        let req = device.get_memory_req((&*buffer).into()).unwrap();
        println!("- Memory requirement = {:?}", req);

        for memory_type in req.memory_types.one_digits() {
            println!("- Trying the memory type '{}'", memory_type);
            println!("  - Creating a heap");

            let heap = device
                .build_heap()
                .size(req.size)
                .memory_type(memory_type)
                .build()
                .unwrap();

            println!("  - Allocating a storage for the buffer");
            let alloc = heap.bind((&*buffer).into())
                .expect("'bind' failed")
                .expect("allocation failed");

            println!("  - Retrieving the pointer to the storage");
            if memory_types[memory_type as usize]
                .caps
                .intersects(gfx::MemoryTypeCaps::HostVisible)
            {
                println!("    Pointer = {:p}", heap.as_ptr(&alloc).unwrap());
            } else {
                println!("    Skipped: Not host visible");
            }

            println!("- Recreating a buffer");
            buffer = utils::UniqueBuffer::new(device, builder.build().unwrap());
        }
    });
}
