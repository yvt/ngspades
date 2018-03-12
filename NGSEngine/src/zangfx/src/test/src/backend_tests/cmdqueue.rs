//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use gfx;
use gfx::prelude::*;
use super::{utils, TestDriver};

pub fn cmdqueue_create<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        use std::cmp::min;

        let queue_families = device.caps().queue_families();
        println!(
            "- {} queue families are defined by the device.",
            queue_families.len()
        );

        for (i, queue_family) in queue_families.iter().enumerate() {
            let count = min(queue_family.count, 16);
            println!(
                "- Creating {} queues for [{}] : {:?}",
                count, i, queue_family
            );
            for k in 0..count {
                println!("  - {} of {}", k + 1, count);
                device
                    .build_cmd_queue()
                    .queue_family(i as _)
                    .build()
                    .unwrap();
            }
        }
    });
}

pub fn cmdqueue_create_fail_missing_queue_family<T: TestDriver>(driver: T) {
    if !driver.is_safe() {
        panic!("this test was skipped because the backend is unsafe");
    }
    driver.for_each_device(&mut |device| {
        device.build_cmd_queue().build().unwrap();
    });
}

pub fn cmdqueue_create_buffer<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        println!("- Creating a command queue");
        let queue: Box<gfx::CmdQueue> = device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a command pool");
        let mut pool = queue.new_cmd_pool().unwrap();

        println!("- Creating a command buffer");
        pool.begin_cmd_buffer().unwrap();
    });
}

pub fn cmdqueue_create_encoder<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let queue_families = device.caps().queue_families();
        println!(
            "- {} queue families are defined by the device.",
            queue_families.len()
        );

        for (i, queue_family) in queue_families.iter().enumerate() {
            println!("- Creating a queue for [{}] : {:?}", i, queue_family);

            println!("- Creating a command queue");
            let queue: Box<gfx::CmdQueue> = device
                .build_cmd_queue()
                .queue_family(i as _)
                .build()
                .unwrap();

            println!("- Creating a command pool");
            let mut pool = queue.new_cmd_pool().unwrap();

            println!("- Creating a command buffer");
            let mut buffer = pool.begin_cmd_buffer().unwrap();

            let caps = queue_family.caps;
            if caps.intersects(gfx::limits::QueueFamilyCaps::Render) {
                println!("- Skipping a render encoder");
                // Starting a render encoder requires other multiple structures
                // to be set up -- let's not do it here
            }
            if caps.intersects(gfx::limits::QueueFamilyCaps::Compute) {
                println!("- Creating a compute encoder");
                buffer.encode_compute();
            }
            if caps.intersects(gfx::limits::QueueFamilyCaps::Copy) {
                println!("- Creating a copy encoder");
                buffer.encode_copy();
            }
        }
    });
}

pub fn cmdqueue_buffer_noop_completes<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        println!("- Creating a command queue");
        let queue: Box<gfx::CmdQueue> = device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a command pool");
        let mut pool = queue.new_cmd_pool().unwrap();

        println!("- Creating a command buffer");
        let mut buffer = pool.begin_cmd_buffer().unwrap();

        println!("- Installing a completion handler");
        let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);

        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        awaiter.wait_until_completed();

        println!("- The execution of the command buffer has completed");
    });
}

pub fn cmdqueue_buffer_noop_completes_dropped_soon<T: TestDriver>(driver: T) {
    use std::mem::drop;
    driver.for_each_device(&mut |device| {
        println!("- Creating a command queue");
        let queue: Box<gfx::CmdQueue> = device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a command pool");
        let mut pool = queue.new_cmd_pool().unwrap();

        println!("- Creating a command buffer");
        let mut buffer = pool.begin_cmd_buffer().unwrap();

        println!("- Installing a completion handler");
        let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);

        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Dropping the command buffer");
        drop(buffer);

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        awaiter.wait_until_completed();

        println!("- The execution of the command buffer has completed");
    });
}

pub fn cmdqueue_buffer_noop_multiple_completes<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        println!("- Creating a command queue");
        let queue: Box<gfx::CmdQueue> = device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a fence");
        let fence = queue.new_fence().unwrap();

        println!("- Creating a barrier");
        let barrier = device
            .build_barrier()
            .global(
                flags![gfx::AccessType::{CopyWrite}],
                flags![gfx::AccessType::{CopyRead}],
            )
            .build()
            .unwrap();

        println!("- Creating a command pool");
        let mut pool = queue.new_cmd_pool().unwrap();

        println!("- Creating a command buffer");
        let mut buffer1 = unsafe { pool.new_cmd_buffer() }.unwrap();
        let mut buffer2 = unsafe { pool.new_cmd_buffer() }.unwrap();

        println!("- Encoding 1");
        {
            let e = buffer1.encode_copy();
            e.update_fence(&fence, gfx::Stage::all());
        }
        println!("- Encoding 2");
        {
            let e = buffer2.encode_copy();
            e.wait_fence(&fence, gfx::Stage::all(), &barrier);
        }

        println!("- Installing a completion handler");
        let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer2);

        println!("- Commiting the command buffer");
        buffer2.commit().unwrap();
        buffer1.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        awaiter.wait_until_completed();

        println!("- The execution of the command buffer has completed");
    });
}

pub fn cmdqueue_buffer_fence_update_wait_completes<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        println!("- Creating a command queue");
        let queue: Box<gfx::CmdQueue> = device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a fence");
        let fence = queue.new_fence().unwrap();

        println!("- Creating a barrier");
        let barrier = device
            .build_barrier()
            .global(
                flags![gfx::AccessType::{CopyWrite}],
                flags![gfx::AccessType::{CopyRead}],
            )
            .build()
            .unwrap();

        println!("- Creating a command pool");
        let mut pool = queue.new_cmd_pool().unwrap();

        println!("- Creating a command buffer");
        let mut buffer = pool.begin_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        // Update and wait on a fence from the same command buffer.
        {
            let e = buffer.encode_copy();
            e.update_fence(&fence, gfx::Stage::all());
        }
        {
            let e = buffer.encode_copy();
            e.wait_fence(&fence, gfx::Stage::all(), &barrier);
        }

        println!("- Installing a completion handler");
        let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);
        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        awaiter.wait_until_completed();

        println!("- The execution of the command buffer has completed");
    });
}
