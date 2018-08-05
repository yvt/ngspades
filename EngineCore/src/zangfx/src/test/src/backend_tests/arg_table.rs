//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{utils, TestDriver};
use include_data::include_data;
use ngsenumflags::flags;
use volatile_view::prelude::*;
use zangfx_base as gfx;
use zangfx_base::prelude::*;
use zangfx_utils::prelude::*;

static SPIRV_READ: ::include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/arg_table_mixed_read.comp.spv"));

fn arg_table_sig_create<T: TestDriver>(driver: T, arg_type: gfx::ArgType) {
    driver.for_each_device(&mut |device| {
        let mut builder = device.build_arg_table_sig();
        builder.arg(0, arg_type).set_len(4);
        builder.build().unwrap();
    });
}

pub fn arg_table_sig_create_image<T: TestDriver>(driver: T) {
    arg_table_sig_create(driver, gfx::ArgType::StorageImage)
}

pub fn arg_table_sig_create_buffer<T: TestDriver>(driver: T) {
    arg_table_sig_create(driver, gfx::ArgType::StorageBuffer)
}

pub fn arg_table_sig_create_sampler<T: TestDriver>(driver: T) {
    arg_table_sig_create(driver, gfx::ArgType::Sampler)
}

fn arg_table<T: TestDriver>(driver: T, arg_type: gfx::ArgType) {
    driver.for_each_device(&mut |device| {
        const TABLE_COUNT: usize = 4;

        let mut builder = device.build_arg_table_sig();
        builder.arg(0, arg_type).set_len(4);
        let sig = builder.build().unwrap();

        println!("- Allocating a pool with deallocation disabled");
        {
            let pool: gfx::ArgPoolRef = device
                .build_arg_pool()
                .reserve_table_sig(TABLE_COUNT, &sig)
                .build()
                .unwrap();

            println!("  - Allocating tables");
            let _tables = pool
                .new_tables(TABLE_COUNT, &sig)
                .unwrap()
                .expect("allocation failed");
            println!("  - Resetting the pool");
            pool.reset().unwrap();
        }

        println!("- Allocating a pool with deallocation enabled");
        {
            let pool: gfx::ArgPoolRef = device
                .build_arg_pool()
                .reserve_table_sig(TABLE_COUNT, &sig)
                .enable_destroy_tables()
                .build()
                .unwrap();

            println!("  - Allocating tables");
            let tables = pool
                .new_tables(TABLE_COUNT, &sig)
                .unwrap()
                .expect("allocation failed");
            println!("  - Deallocating tables");
            pool.destroy_tables(tables.iter().collect::<Vec<_>>().as_slice())
                .unwrap();

            println!("  - Allocating tables");
            let tables = pool
                .new_tables(TABLE_COUNT, &sig)
                .unwrap()
                .expect("allocation failed");
            println!("  - Deallocating tables");
            pool.destroy_tables(tables.iter().collect::<Vec<_>>().as_slice())
                .unwrap();
        }
    });
}

pub fn arg_table_image<T: TestDriver>(driver: T) {
    arg_table(driver, gfx::ArgType::StorageImage)
}

pub fn arg_table_buffer<T: TestDriver>(driver: T) {
    arg_table(driver, gfx::ArgType::StorageBuffer)
}

pub fn arg_table_sampler<T: TestDriver>(driver: T) {
    arg_table(driver, gfx::ArgType::Sampler)
}

/// Create an argument table containg various kinds of arguments and see if
/// it can be used successfully.
pub fn arg_table_mixed_read<T: TestDriver>(driver: T) {
    driver.for_each_compute_queue(&mut |device, qf| {
        println!("- Creating a command queue");
        let queue = device
            .build_cmd_queue()
            .queue_family(qf)
            .label("Main queue")
            .build()
            .unwrap();

        println!("- Creating an argument table signature");
        let arg_table_sig = {
            let mut builder = device.build_arg_table_sig();
            builder.arg(0, gfx::ArgType::StorageBuffer);
            builder.arg(1, gfx::ArgType::SampledImage);
            builder.arg(2, gfx::ArgType::UniformBuffer);
            builder.arg(3, gfx::ArgType::SampledImage);
            builder.arg(4, gfx::ArgType::UniformBuffer);
            builder.arg(5, gfx::ArgType::SampledImage);
            builder.arg(6, gfx::ArgType::UniformBuffer);
            builder.arg(7, gfx::ArgType::SampledImage);
            builder.arg(8, gfx::ArgType::UniformBuffer);
            builder.arg(9, gfx::ArgType::Sampler);
            builder.build().unwrap()
        };

        println!("- Creating a root signature");
        let root_sig = device
            .build_root_sig()
            .arg_table(0, &arg_table_sig)
            .build()
            .unwrap();

        println!("- Creating a buffer");
        let buffer = device
            .build_buffer()
            .size(4096)
            .usage(flags![gfx::BufferUsage::{Storage | Uniform}])
            .queue(&queue)
            .build()
            .unwrap();

        println!("- Creating images");
        let mut builder = device.build_image();
        builder.format(<u8>::as_rgba_norm()).queue(&queue);

        let image_cube = builder.extents_cube(1).build().unwrap();

        let image_2d = builder.extents(&[1, 1]).build().unwrap();

        let image_3d = builder.extents(&[1, 1, 1]).build().unwrap();

        let image_2ds = builder
            .extents(&[1, 1])
            .num_layers(Some(1))
            .build()
            .unwrap();

        println!("- Computing the memory requirements for the heap");
        let valid_memory_types = buffer.get_memory_req().unwrap().memory_types;
        let memory_type = utils::choose_memory_type(
            device,
            valid_memory_types,
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
            flags![gfx::MemoryTypeCaps::{HostVisible | HostCoherent}],
        );
        println!("  Memory Type = {}", memory_type);

        println!("- Allocating memory");
        {
            let heap = device.global_heap(memory_type);
            assert!(heap.bind((&buffer).into()).unwrap());
        }

        println!("- Computing the memory requirements for the image heap");
        let valid_memory_types = image_2d.get_memory_req().unwrap().memory_types;
        let memory_type = utils::choose_memory_type(
            device,
            valid_memory_types,
            flags![gfx::MemoryTypeCaps::{}],
            flags![gfx::MemoryTypeCaps::{}],
        );
        println!("  Memory Type = {}", memory_type);

        println!("- Allocating memory");
        {
            let heap = device.global_heap(memory_type);
            assert!(heap.bind((&image_2d).into()).unwrap());
            assert!(heap.bind((&image_2ds).into()).unwrap());
            assert!(heap.bind((&image_3d).into()).unwrap());
            assert!(heap.bind((&image_cube).into()).unwrap());
        }

        println!("- Retrieving a pointer to the allocated buffer");
        let buffer_view = buffer.as_volatile::<u32>().unwrap();
        println!("  Pointer = {:p}", buffer_view.as_ptr());

        println!("- Creating a sampler");
        let sampler = device.build_sampler().build().unwrap();

        println!("- Storing the shader inputs");
        const INPUT1_OFFSET: usize = 256;
        const INPUT2_OFFSET: usize = 512;
        const INPUT3_OFFSET: usize = 768;
        const INPUT4_OFFSET: usize = 1024;
        buffer_view[INPUT1_OFFSET / 4].store(114);
        buffer_view[INPUT2_OFFSET / 4].store(514);
        buffer_view[INPUT3_OFFSET / 4].store(810);
        buffer_view[INPUT4_OFFSET / 4].store(1919);

        println!("- Creating a library");
        let library = device.new_library(SPIRV_READ.as_u32_slice()).unwrap();

        println!("- Allocating a pool with deallocation disabled");
        let pool: gfx::ArgPoolRef = device
            .build_arg_pool()
            .reserve_table_sig(1, &arg_table_sig)
            .queue(&queue)
            .build()
            .unwrap();

        println!("  - Allocating an argument table");
        let arg_table = pool
            .new_table(&arg_table_sig)
            .unwrap()
            .expect("allocation failed");

        let range = |start| start as u64..(start + 256) as u64;
        println!("- Writing the argument table");
        device
            .update_arg_table(
                &arg_table_sig,
                &pool,
                &arg_table,
                &[
                    (0, 0, [(range(0), &buffer)][..].into()),
                    (2, 0, [(range(INPUT1_OFFSET), &buffer)][..].into()),
                    (4, 0, [(range(INPUT2_OFFSET), &buffer)][..].into()),
                    (6, 0, [(range(INPUT3_OFFSET), &buffer)][..].into()),
                    (8, 0, [(range(INPUT4_OFFSET), &buffer)][..].into()),
                    (1, 0, [&image_cube][..].into()),
                    (3, 0, [&image_2d][..].into()),
                    (5, 0, [&image_3d][..].into()),
                    (7, 0, [&image_2ds][..].into()),
                    (9, 0, [&sampler][..].into()),
                ],
            ).unwrap();

        println!("- Creating a pipeline");
        let pipeline = device
            .build_compute_pipeline()
            .compute_shader(&library, "main")
            .root_sig(&root_sig)
            .build()
            .unwrap();

        println!("- Creating a command buffer");
        let mut cmd_buffer = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e: &mut dyn gfx::ComputeCmdEncoder = cmd_buffer.encode_compute();
            e.use_resource_read_write(&buffer);
            e.bind_pipeline(&pipeline);
            e.bind_arg_table(0, &[(&pool, &arg_table)]);
            e.dispatch(&[]);
        }
        cmd_buffer.host_barrier(
            flags![gfx::AccessType::{ComputeWrite}],
            &[(range(0), &buffer)],
        );

        println!("- Installing a completion handler");
        let awaiter = utils::CmdBufferAwaiter::new(&mut *cmd_buffer);

        println!("- Commiting the command buffer");
        cmd_buffer.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        awaiter.wait_until_completed();

        println!("- Reading back the result");
        let ret: Vec<_> = buffer_view.load();
        assert_eq!(ret[0], ret[INPUT1_OFFSET / 4]);
        assert_eq!(ret[1], ret[INPUT2_OFFSET / 4]);
        assert_eq!(ret[2], ret[INPUT3_OFFSET / 4]);
        assert_eq!(ret[3], ret[INPUT4_OFFSET / 4]);
        assert_eq!(ret[4], 0xdeadbeef);
    });
}
