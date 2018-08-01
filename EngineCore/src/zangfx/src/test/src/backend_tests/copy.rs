//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{utils, TestDriver};
use ngsenumflags::flags;
use volatile_view::prelude::*;
use zangfx_base as gfx;
use zangfx_base::prelude::*;
use zangfx_utils::prelude::*;

pub fn copy_fill_buffer<T: TestDriver>(driver: T) {
    driver.for_each_copy_queue(&mut |device, qf| {
        println!("- Creating a command queue");
        let queue = device
            .build_cmd_queue()
            .queue_family(qf)
            .label("Main queue")
            .build()
            .unwrap();

        println!("- Creating a buffer");
        let buffer1 = device
            .build_buffer()
            .label("Buffer 1")
            .size(65536)
            .usage(flags![gfx::BufferUsage::{CopyWrite}])
            .queue(&queue)
            .build()
            .unwrap();

        println!("- Computing the memory requirements for the heap");
        let valid_memory_types = buffer1.get_memory_req().unwrap().memory_types;
        let memory_type = utils::choose_memory_type(
            device,
            valid_memory_types,
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
        );
        println!("  Memory Type = {}", memory_type);

        println!("- Allocating memory");
        device
            .global_heap(memory_type)
            .bind((&buffer1).into())
            .unwrap();

        println!("- Retrieving pointers to the allocated buffer");
        let buffer1_view = buffer1.as_bytes_volatile();
        println!("  Ptr = {:p}", buffer1_view.as_ptr());

        println!("- Storing the input");
        for x in buffer1_view {
            x.store(0);
        }

        println!("- Creating a command buffer");
        let mut buffer = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e: &mut dyn gfx::CopyCmdEncoder = buffer.encode_copy();
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
        let ret: Vec<_> = buffer1_view.load();
        assert_eq!(ret[0..400], [0x12u8; 400][..]);
        assert_eq!(ret[400..800], [0u8; 400][..]);
        assert_eq!(ret[800..1200], [0xafu8; 400][..]);
    });
}

pub fn copy_copy_buffer<T: TestDriver>(driver: T) {
    driver.for_each_copy_queue(&mut |device, qf| {
        println!("- Creating a command queue");
        let queue = device
            .build_cmd_queue()
            .queue_family(qf)
            .label("Main queue")
            .build()
            .unwrap();

        println!("- Creating buffers");
        let buffer1 = device
            .build_buffer()
            .label("Buffer 1")
            .size(65536)
            .usage(flags![gfx::BufferUsage::{CopyRead}])
            .queue(&queue)
            .build()
            .unwrap();
        let buffer2 = device
            .build_buffer()
            .label("Buffer 2")
            .size(65536)
            .usage(flags![gfx::BufferUsage::{CopyWrite}])
            .queue(&queue)
            .build()
            .unwrap();

        println!("- Computing the memory requirements for the heap");
        let valid_memory_types = buffer1.get_memory_req().unwrap().memory_types;
        let memory_type = utils::choose_memory_type(
            device,
            valid_memory_types,
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
        );
        println!("  Memory Type = {}", memory_type);

        println!("- Allocating memory");
        let heap = device.global_heap(memory_type);
        heap.bind((&buffer1).into()).unwrap();
        heap.bind((&buffer2).into()).unwrap();

        println!("- Retrieving pointers to the allocated buffer");
        let buffer1_view = buffer1.as_bytes_volatile();
        let buffer2_view = buffer2.as_bytes_volatile();
        println!(
            "  Input = {:p}, Output = {:p}",
            buffer1_view.as_ptr(),
            buffer2_view.as_ptr()
        );

        println!("- Storing the input");
        let data = "The quick brown 𠮷野家 jumped over the lazy ま・つ・や.".as_bytes();
        buffer1_view[4..4 + data.len()].copy_from_slice(&data);
        for x in buffer2_view {
            x.store(0);
        }

        println!("- Creating a command buffer");
        let mut buffer = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e: &mut dyn gfx::CopyCmdEncoder = buffer.encode_copy();
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
        let ret: Vec<_> = buffer2_view.load();
        assert_eq!(ret[0..1200], [0u8; 1200][..]);
        assert_eq!(ret[1200..1200 + data.len() / 2], data[0..data.len() / 2]);
        assert_eq!(
            ret[1200 + data.len() / 2..1200 + data.len() / 2 + 1200],
            [0u8; 1200][..]
        );
    });
}
