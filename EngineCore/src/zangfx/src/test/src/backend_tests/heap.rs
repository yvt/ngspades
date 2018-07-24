//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::TestDriver;
use ngsenumflags::flags;
use zangfx_base as gfx;
use zangfx_common::BinaryInteger;

pub fn heap_dynamic_create<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let memory_types = device.caps().memory_types();
        println!(
            "- {} memory types are defined by the device.",
            memory_types.len()
        );

        for (i, memory_type) in memory_types.iter().enumerate() {
            println!("- Creating a heap for [{}] : {:?}", i, memory_type);
            device
                .build_dynamic_heap()
                .size(256)
                .memory_type(i as _)
                .build()
                .unwrap();
        }
    });
}

pub fn heap_dynamic_create_fail_zero_size<T: TestDriver>(driver: T) {
    if !driver.is_safe() {
        panic!("this test was skipped because the backend is unsafe");
    }
    driver.for_each_device(&mut |device| {
        device.build_dynamic_heap().memory_type(0).build().unwrap();
    });
}

pub fn heap_dynamic_create_fail_missing_memory_type<T: TestDriver>(driver: T) {
    if !driver.is_safe() {
        panic!("this test was skipped because the backend is unsafe");
    }
    driver.for_each_device(&mut |device| {
        device.build_dynamic_heap().build().unwrap();
    });
}

pub fn heap_dynamic_alloc_buffer<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let memory_types = device.caps().memory_types();

        let mut builder = device.build_buffer();
        builder
            .size(1001)
            .usage(flags![gfx::BufferUsage::{CopyRead | CopyWrite}]);

        println!("- Creating a buffer");
        let mut buffer = builder.build().unwrap();

        println!("- Querying the memory requirement for the buffer");
        let req = buffer.get_memory_req().unwrap();
        println!("- Memory requirement = {:?}", req);

        for memory_type in req.memory_types.one_digits() {
            println!("- Trying the memory type '{}'", memory_type);
            println!("  - Creating a heap");

            let heap = device
                .build_dynamic_heap()
                .size(req.size)
                .memory_type(memory_type)
                .build()
                .unwrap();

            println!("  - Allocating a storage for the buffer");
            let alloc = heap.bind((&buffer).into()).expect("'bind' failed");
            assert!(alloc, "allocation failed");

            println!("  - Retrieving the pointer to the storage");
            if memory_types[memory_type as usize]
                .caps
                .intersects(gfx::MemoryTypeCaps::HostVisible)
            {
                println!("    Pointer = {:p}", buffer.as_ptr());
            } else {
                println!("    Skipped: Not host visible");
            }

            println!("- Recreating a buffer");
            buffer = builder.build().unwrap();
        }
    });
}

pub fn heap_dynamic_alloc_image<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let mut builder = device.build_image();
        builder
            .extents(&[256, 256])
            .format(gfx::ImageFormat::SrgbRgba8);

        println!("- Creating an image");
        let mut image = builder.build().unwrap();

        println!("- Querying the memory requirement for the image");
        let req = image.get_memory_req().unwrap();
        println!("- Memory requirement = {:?}", req);

        for memory_type in req.memory_types.one_digits() {
            println!("- Trying the memory type '{}'", memory_type);
            println!("  - Creating a heap");

            let heap = device
                .build_dynamic_heap()
                .size(req.size)
                .memory_type(memory_type)
                .build()
                .unwrap();

            println!("  - Allocating a storage for the image");
            let alloc = heap.bind((&image).into()).expect("'bind' failed");
            assert!(alloc, "allocation failed");

            println!("- Recreating a image");
            image = builder.build().unwrap();
        }
    });
}

pub fn heap_dedicated_create_fail_zero_size<T: TestDriver>(driver: T) {
    if !driver.is_safe() {
        panic!("this test was skipped because the backend is unsafe");
    }
    driver.for_each_device(&mut |device| {
        device
            .build_dedicated_heap()
            .memory_type(0)
            .build()
            .unwrap();
    });
}

pub fn heap_dedicated_create_fail_missing_memory_type<T: TestDriver>(driver: T) {
    if !driver.is_safe() {
        panic!("this test was skipped because the backend is unsafe");
    }
    driver.for_each_device(&mut |device| {
        device.build_dedicated_heap().build().unwrap();
    });
}

pub fn heap_dedicated_alloc_buffer<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let memory_types = device.caps().memory_types();

        let mut builder = device.build_buffer();
        builder
            .size(1001)
            .usage(flags![gfx::BufferUsage::{CopyRead | CopyWrite}]);

        println!("- Creating a buffer");
        let mut buffer = builder.build().unwrap();

        println!("- Querying the memory requirement for the buffer");
        let req = buffer.get_memory_req().unwrap();
        println!("- Memory requirement = {:?}", req);

        for memory_type in req.memory_types.one_digits() {
            println!("- Trying the memory type '{}'", memory_type);
            println!("  - Creating a heap");

            let _heap = {
                let mut builder = device.build_dedicated_heap();
                builder.memory_type(memory_type);
                builder.bind((&buffer).into());
                builder.build().unwrap()
            };

            println!("  - Retrieving the pointer to the storage");
            if memory_types[memory_type as usize]
                .caps
                .intersects(gfx::MemoryTypeCaps::HostVisible)
            {
                println!("    Pointer = {:p}", buffer.as_ptr());
            } else {
                println!("    Skipped: Not host visible");
            }

            println!("- Recreating a buffer");
            buffer = builder.build().unwrap();
        }
    });
}

pub fn heap_dedicated_alloc_image<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let mut builder = device.build_image();
        builder
            .extents(&[256, 256])
            .format(gfx::ImageFormat::SrgbRgba8);

        println!("- Creating an image");
        let mut image = builder.build().unwrap();

        println!("- Querying the memory requirement for the image");
        let req = image.get_memory_req().unwrap();
        println!("- Memory requirement = {:?}", req);

        for memory_type in req.memory_types.one_digits() {
            println!("- Trying the memory type '{}'", memory_type);
            println!("  - Creating a heap");

            let _heap = {
                let mut builder = device.build_dedicated_heap();
                builder.memory_type(memory_type);
                builder.bind((&image).into());
                builder.build().unwrap()
            };

            println!("- Recreating a image");
            image = builder.build().unwrap();
        }
    });
}
