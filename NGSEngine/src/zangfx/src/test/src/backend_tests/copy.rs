//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::slice::from_raw_parts_mut;
use gfx;
use gfx::prelude::*;
use super::{utils, TestDriver};

pub fn copy_fill_buffer<T: TestDriver>(driver: T) {
    driver.for_each_copy_queue(&mut |device, qf| {
        println!("- Creating a buffer");
        let buffer1 = utils::UniqueBuffer::new(
            device,
            device
                .build_buffer()
                .label("Buffer 1")
                .size(65536)
                .usage(flags![gfx::BufferUsage::{CopyWrite}])
                .build()
                .unwrap(),
        );

        println!("- Computing the memory requirements for the heap");
        let valid_memory_types = device
            .get_memory_req((&*buffer1).into())
            .unwrap()
            .memory_types;
        let memory_type = utils::choose_memory_type(
            device,
            valid_memory_types,
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
        );
        println!("  Memory Type = {}", memory_type);

        println!("- Creating a heap");
        let heap: Box<gfx::Heap> = {
            let mut builder = device.build_dedicated_heap();
            builder.memory_type(memory_type).label("Buffer heap");
            builder.prebind((&*buffer1).into());
            builder.build().unwrap()
        };

        println!("- Retrieving pointers to the allocated buffer");
        let buffer1_ptr = unsafe {
            let alloc = heap.bind((&*buffer1).into()).unwrap().unwrap();
            let ptr = heap.as_ptr(&alloc).unwrap();
            from_raw_parts_mut(ptr as *mut u8, 65536)
        };
        println!("  Ptr = {:p}", buffer1_ptr);

        println!("- Storing the input");
        for x in buffer1_ptr.iter_mut() {
            *x = 0;
        }

        println!("- Creating a command queue");
        let queue = device
            .build_cmd_queue()
            .queue_family(qf)
            .label("Main queue")
            .build()
            .unwrap();

        println!("- Creating a command buffer");
        let mut buffer: Box<gfx::CmdBuffer> = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e: &mut gfx::CopyCmdEncoder = buffer.encode_copy();
            e.begin_debug_group("Convolution");
            e.fill_buffer(&buffer1, 0..400, 0x12);
            e.fill_buffer(&buffer1, 800..1200, 0xaf);
            e.end_debug_group();
        }
        buffer.host_barrier(
            flags![gfx::AccessType::{CopyWrite}],
            &[(0..65536, &buffer1)],
        );

        println!("- Installing a completion handler");
        let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);

        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        awaiter.wait_until_completed();

        println!("- Comparing the result");
        assert_eq!(buffer1_ptr[0..400], [0x12u8; 400][..]);
        assert_eq!(buffer1_ptr[400..800], [0u8; 400][..]);
        assert_eq!(buffer1_ptr[800..1200], [0xafu8; 400][..]);
    });
}

pub fn copy_copy_buffer<T: TestDriver>(driver: T) {
    driver.for_each_copy_queue(&mut |device, qf| {
        println!("- Creating buffers");
        let buffer1 = utils::UniqueBuffer::new(
            device,
            device
                .build_buffer()
                .label("Buffer 1")
                .size(65536)
                .usage(flags![gfx::BufferUsage::{CopyRead}])
                .build()
                .unwrap(),
        );
        let buffer2 = utils::UniqueBuffer::new(
            device,
            device
                .build_buffer()
                .label("Buffer 2")
                .size(65536)
                .usage(flags![gfx::BufferUsage::{CopyWrite}])
                .build()
                .unwrap(),
        );

        println!("- Computing the memory requirements for the heap");
        let valid_memory_types = device
            .get_memory_req((&*buffer1).into())
            .unwrap()
            .memory_types;
        let memory_type = utils::choose_memory_type(
            device,
            valid_memory_types,
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
        );
        println!("  Memory Type = {}", memory_type);

        println!("- Creating a heap");
        let heap: Box<gfx::Heap> = {
            let mut builder = device.build_dedicated_heap();
            builder.memory_type(memory_type).label("Buffer heap");
            builder.prebind((&*buffer1).into());
            builder.prebind((&*buffer2).into());
            builder.build().unwrap()
        };

        println!("- Retrieving pointers to the allocated buffer");
        let buffer1_ptr = unsafe {
            let alloc = heap.bind((&*buffer1).into()).unwrap().unwrap();
            let ptr = heap.as_ptr(&alloc).unwrap();
            from_raw_parts_mut(ptr as *mut u8, 65536)
        };
        let buffer2_ptr = unsafe {
            let alloc = heap.bind((&*buffer2).into()).unwrap().unwrap();
            let ptr = heap.as_ptr(&alloc).unwrap();
            from_raw_parts_mut(ptr as *mut u8, 65536)
        };
        println!("  Input = {:p}, Output = {:p}", buffer1_ptr, buffer2_ptr);

        println!("- Storing the input");
        let data = "The quick brown 𠮷野家 jumped over the lazy ま・つ・や.".as_bytes();
        buffer1_ptr[4..4 + data.len()].copy_from_slice(&data);
        for x in buffer2_ptr.iter_mut() {
            *x = 0;
        }

        println!("- Creating a command queue");
        let queue = device
            .build_cmd_queue()
            .queue_family(qf)
            .label("Main queue")
            .build()
            .unwrap();

        println!("- Creating a command buffer");
        let mut buffer: Box<gfx::CmdBuffer> = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e: &mut gfx::CopyCmdEncoder = buffer.encode_copy();
            e.begin_debug_group("Convolution");
            e.copy_buffer(&buffer1, 4, &buffer2, 1200, data.len() as u64 / 2);
            e.end_debug_group();
        }
        buffer.host_barrier(
            flags![gfx::AccessType::{CopyWrite}],
            &[(0..65536, &buffer2)],
        );

        println!("- Installing a completion handler");
        let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);

        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        awaiter.wait_until_completed();

        println!("- Comparing the result");
        assert_eq!(buffer2_ptr[0..1200], [0u8; 1200][..]);
        assert_eq!(
            buffer2_ptr[1200..1200 + data.len() / 2],
            data[0..data.len() / 2]
        );
        assert_eq!(
            buffer2_ptr[1200 + data.len() / 2..1200 + data.len() / 2 + 1200],
            [0u8; 1200][..]
        );
    });
}
