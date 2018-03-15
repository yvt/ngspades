//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::slice::from_raw_parts_mut;
use gfx;
use gfx::prelude::*;
use super::{utils, TestDriver};

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
            let mut pool: Box<gfx::ArgPool> = device
                .build_arg_pool()
                .reserve_table_sig(TABLE_COUNT, &sig)
                .build()
                .unwrap();

            println!("  - Allocating tables");
            let _tables = pool.new_tables(TABLE_COUNT, &sig)
                .unwrap()
                .expect("allocation failed");
            println!("  - Resetting the pool");
            pool.reset().unwrap();
        }

        println!("- Allocating a pool with deallocation enabled");
        {
            let mut pool: Box<gfx::ArgPool> = device
                .build_arg_pool()
                .reserve_table_sig(TABLE_COUNT, &sig)
                .enable_destroy_tables()
                .build()
                .unwrap();

            println!("  - Allocating tables");
            let tables = pool.new_tables(TABLE_COUNT, &sig)
                .unwrap()
                .expect("allocation failed");
            println!("  - Deallocating tables");
            pool.destroy_tables(tables.iter().collect::<Vec<_>>().as_slice())
                .unwrap();

            println!("  - Allocating tables");
            let tables = pool.new_tables(TABLE_COUNT, &sig)
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
            .build()
            .unwrap();
        let buffer = utils::UniqueBuffer::new(device, buffer);

        println!("- Creating images");
        let mut builder = device.build_image();
        builder.format(<u8>::as_rgba_norm());

        let image_cube = builder.extents_cube(1).build().unwrap();
        let image_cube = utils::UniqueImage::new(device, image_cube);

        let image_2d = builder.extents(&[1, 1]).build().unwrap();
        let image_2d = utils::UniqueImage::new(device, image_2d);

        let image_3d = builder.extents(&[1, 1, 1]).build().unwrap();
        let image_3d = utils::UniqueImage::new(device, image_3d);

        let image_2ds = builder
            .extents(&[1, 1])
            .num_layers(Some(1))
            .build()
            .unwrap();
        let image_2ds = utils::UniqueImage::new(device, image_2ds);

        println!("- Computing the memory requirements for the heap");
        let valid_memory_types = device
            .get_memory_req((&*buffer).into())
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
            builder.prebind((&*buffer).into());
            builder.build().unwrap()
        };

        println!("- Computing the memory requirements for the image heap");
        let valid_memory_types = device
            .get_memory_req((&*image_2d).into())
            .unwrap()
            .memory_types;
        let memory_type = utils::choose_memory_type(
            device,
            valid_memory_types,
            flags![gfx::MemoryTypeCaps::{}],
            flags![gfx::MemoryTypeCaps::{}],
        );
        println!("  Memory Type = {}", memory_type);

        println!("- Creating an image heap");
        let image_heap: Box<gfx::Heap> = {
            let mut builder = device.build_dedicated_heap();
            builder.memory_type(memory_type).label("Image heap");
            builder.prebind((&*image_2d).into());
            builder.prebind((&*image_2ds).into());
            builder.prebind((&*image_3d).into());
            builder.prebind((&*image_cube).into());
            builder.build().unwrap()
        };

        println!("- Retrieving a pointer to the allocated buffer");
        let ptr = unsafe {
            let alloc = heap.bind((&*buffer).into()).unwrap().unwrap();
            let ptr = heap.as_ptr(&alloc).unwrap();
            from_raw_parts_mut(ptr as *mut u32, 1024)
        };
        println!("  Pointer = {:p}", ptr);

        println!("- Finalizing the memory binding of the images");
        image_heap.bind((&*image_2d).into()).unwrap().unwrap();
        image_heap.bind((&*image_2ds).into()).unwrap().unwrap();
        image_heap.bind((&*image_3d).into()).unwrap().unwrap();
        image_heap.bind((&*image_cube).into()).unwrap().unwrap();

        println!("- Creating image views");
        let layout = gfx::ImageLayout::ShaderRead;

        let image_2d_view = device.new_image_view(&image_2d, layout).unwrap();
        let image_2d_view = utils::UniqueImageView::new(device, image_2d_view);

        let image_2ds_view = device.new_image_view(&image_2ds, layout).unwrap();
        let image_2ds_view = utils::UniqueImageView::new(device, image_2ds_view);

        let image_3d_view = device.new_image_view(&image_3d, layout).unwrap();
        let image_3d_view = utils::UniqueImageView::new(device, image_3d_view);

        let image_cube_view = device.new_image_view(&image_cube, layout).unwrap();
        let image_cube_view = utils::UniqueImageView::new(device, image_cube_view);

        println!("- Creating a sampler");
        let mut builder = device.build_sampler();
        let sampler = utils::UniqueSampler::new(device, builder.build().unwrap());

        println!("- Storing the shader inputs");
        const INPUT1_OFFSET: usize = 256;
        const INPUT2_OFFSET: usize = 512;
        const INPUT3_OFFSET: usize = 768;
        const INPUT4_OFFSET: usize = 1024;
        ptr[INPUT1_OFFSET / 4] = 114;
        ptr[INPUT2_OFFSET / 4] = 514;
        ptr[INPUT3_OFFSET / 4] = 810;
        ptr[INPUT4_OFFSET / 4] = 1919;

        println!("- Creating a command queue");
        let queue = device
            .build_cmd_queue()
            .queue_family(qf)
            .label("Main queue")
            .build()
            .unwrap();

        println!("- Creating a library");
        let library = device.new_library(SPIRV_READ.as_u32_slice()).unwrap();

        println!("- Allocating a pool with deallocation disabled");
        let mut pool: Box<gfx::ArgPool> = device
            .build_arg_pool()
            .reserve_table_sig(1, &arg_table_sig)
            .build()
            .unwrap();

        println!("  - Allocating an argument table");
        let arg_table = pool.new_table(&arg_table_sig)
            .unwrap()
            .expect("allocation failed");

        let range = |start| start as u64..(start + 256) as u64;
        println!("- Writing the argument table");
        device
            .update_arg_table(
                &arg_table_sig,
                &arg_table,
                &[
                    (0, 0, [(range(0), &*buffer)][..].into()),
                    (2, 0, [(range(INPUT1_OFFSET), &*buffer)][..].into()),
                    (4, 0, [(range(INPUT2_OFFSET), &*buffer)][..].into()),
                    (6, 0, [(range(INPUT3_OFFSET), &*buffer)][..].into()),
                    (8, 0, [(range(INPUT4_OFFSET), &*buffer)][..].into()),
                    (1, 0, [&*image_cube_view][..].into()),
                    (3, 0, [&*image_2d_view][..].into()),
                    (5, 0, [&*image_3d_view][..].into()),
                    (7, 0, [&*image_2ds_view][..].into()),
                    (9, 0, [&*sampler][..].into()),
                ],
            )
            .unwrap();

        println!("- Creating a pipeline");
        let pipeline = device
            .build_compute_pipeline()
            .compute_shader(&library, "main")
            .root_sig(&root_sig)
            .build()
            .unwrap();

        println!("- Creating a command pool");
        let mut pool = queue.new_cmd_pool().unwrap();

        println!("- Creating a command buffer");
        let mut cmd_buffer: gfx::SafeCmdBuffer = pool.begin_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e: &mut gfx::ComputeCmdEncoder = cmd_buffer.encode_compute();
            e.use_resource(gfx::ResourceUsage::Write, &[(&*buffer).into()]);
            e.bind_pipeline(&pipeline);
            e.bind_arg_table(0, &[&arg_table]);
            e.dispatch(&[]);
        }
        cmd_buffer.host_barrier(
            flags![gfx::AccessType::{ComputeWrite}],
            &[(range(0), &*buffer)],
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
        assert_eq!(ptr[0], ptr[INPUT1_OFFSET / 4]);
        assert_eq!(ptr[1], ptr[INPUT2_OFFSET / 4]);
        assert_eq!(ptr[2], ptr[INPUT3_OFFSET / 4]);
        assert_eq!(ptr[3], ptr[INPUT4_OFFSET / 4]);
        assert_eq!(ptr[4], 0xdeadbeef);
    });
}
