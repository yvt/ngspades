//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use base;
use super::TestDriver;

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
        let queue: Box<base::command::CmdQueue> =
            device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a command buffer");
        queue.new_cmd_buffer().unwrap();
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
            let queue: Box<base::command::CmdQueue> = device
                .build_cmd_queue()
                .queue_family(i as _)
                .build()
                .unwrap();

            println!("- Creating a command buffer");
            let mut buffer: Box<base::command::CmdBuffer> = queue.new_cmd_buffer().unwrap();

            let caps = queue_family.caps;
            if caps.intersects(base::limits::QueueFamilyCaps::Render) {
                println!("- Skipping a render encoder");
                // Starting a render encoder requires other multiple structures
                // to be set up -- let's not do it here
            }
            if caps.intersects(base::limits::QueueFamilyCaps::Compute) {
                println!("- Creating a compute encoder");
                buffer.encode_compute();
            }
            if caps.intersects(base::limits::QueueFamilyCaps::Copy) {
                println!("- Creating a copy encoder");
                buffer.encode_copy();
            }
        }
    });
}

pub fn cmdqueue_buffer_noop_completes<T: TestDriver>(driver: T) {
    use std::sync::mpsc;
    use std::time::Duration;
    driver.for_each_device(&mut |device| {
        println!("- Creating a command queue");
        let queue: Box<base::command::CmdQueue> =
            device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a command buffer");
        let mut buffer: Box<base::command::CmdBuffer> = queue.new_cmd_buffer().unwrap();

        println!("- Installing a completion handler");
        let (send, recv) = mpsc::channel();
        buffer.on_complete(Box::new(move || {
            let _ = send.send(());
        }));
        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        recv.recv_timeout(Duration::from_millis(1000)).unwrap();

        println!("- The execution of the command buffer has completed");
    });
}

pub fn cmdqueue_buffer_noop_completes_dropped_soon<T: TestDriver>(driver: T) {
    use std::sync::mpsc;
    use std::time::Duration;
    use std::mem::drop;
    driver.for_each_device(&mut |device| {
        println!("- Creating a command queue");
        let queue: Box<base::command::CmdQueue> =
            device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a command buffer");
        let mut buffer: Box<base::command::CmdBuffer> = queue.new_cmd_buffer().unwrap();

        println!("- Installing a completion handler");
        let (send, recv) = mpsc::channel();
        buffer.on_complete(Box::new(move || {
            let _ = send.send(());
        }));
        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Dropping the command buffer");
        drop(buffer);

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        recv.recv_timeout(Duration::from_millis(1000)).unwrap();

        println!("- The execution of the command buffer has completed");
    });
}

pub fn cmdqueue_buffer_noop_multiple_completes<T: TestDriver>(driver: T) {
    use std::sync::mpsc;
    use std::time::Duration;
    driver.for_each_device(&mut |device| {
        println!("- Creating a command queue");
        let queue: Box<base::command::CmdQueue> =
            device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a fence");
        let fence = queue.new_fence().unwrap();

        println!("- Creating a barrier");
        let barrier = device
            .build_barrier()
            .global(
                flags![base::AccessType::{CopyWrite}],
                flags![base::AccessType::{CopyRead}],
            )
            .build()
            .unwrap();

        println!("- Creating a command buffer");
        let mut buffer1: Box<base::command::CmdBuffer> = queue.new_cmd_buffer().unwrap();
        let mut buffer2: Box<base::command::CmdBuffer> = queue.new_cmd_buffer().unwrap();

        println!("- Encoding 1");
        {
            let e = buffer1.encode_copy();
            e.update_fence(&fence, flags![base::Stage::{All}]);
        }
        println!("- Encoding 2");
        {
            let e = buffer2.encode_copy();
            e.wait_fence(
                &fence,
                flags![base::Stage::{All}],
                flags![base::Stage::{All}],
                &barrier,
            );
        }

        println!("- Installing a completion handler");
        let (send, recv) = mpsc::channel();
        buffer2.on_complete(Box::new(move || {
            let _ = send.send(());
        }));
        println!("- Commiting the command buffer");
        buffer2.commit().unwrap();
        buffer1.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        recv.recv_timeout(Duration::from_millis(1000)).unwrap();

        println!("- The execution of the command buffer has completed");
    });
}

pub fn cmdqueue_buffer_fence_update_wait_completes<T: TestDriver>(driver: T) {
    use std::sync::mpsc;
    use std::time::Duration;
    driver.for_each_device(&mut |device| {
        println!("- Creating a command queue");
        let queue: Box<base::command::CmdQueue> =
            device.build_cmd_queue().queue_family(0).build().unwrap();

        println!("- Creating a fence");
        let fence = queue.new_fence().unwrap();

        println!("- Creating a barrier");
        let barrier = device
            .build_barrier()
            .global(
                flags![base::AccessType::{CopyWrite}],
                flags![base::AccessType::{CopyRead}],
            )
            .build()
            .unwrap();

        println!("- Creating a command buffer");
        let mut buffer: Box<base::command::CmdBuffer> = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e = buffer.encode_copy();
            // Update and wait on a fence from the same command buffer.
            e.update_fence(&fence, flags![base::Stage::{All}]);
            e.wait_fence(
                &fence,
                flags![base::Stage::{All}],
                flags![base::Stage::{All}],
                &barrier,
            );
        }

        println!("- Installing a completion handler");
        let (send, recv) = mpsc::channel();
        buffer.on_complete(Box::new(move || {
            let _ = send.send(());
        }));
        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        recv.recv_timeout(Duration::from_millis(1000)).unwrap();

        println!("- The execution of the command buffer has completed");
    });
}
