//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::mem::size_of_val;
use std::slice::from_raw_parts_mut;
use gfx;
use gfx::prelude::*;
use super::{utils, TestDriver};

static SPIRV_CONV: ::include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/compute_conv1.comp.spv"));

/// Performs a convolution using a compute shader.
pub fn compute_conv1<T: TestDriver>(driver: T) {
    driver.for_each_compute_queue(&mut |device, qf| {
        let binding_redundant = 0; // unused -- evoke possible issue in arg table handling
        let binding_param = 1;
        let binding_input = 2;
        let binding_output = 3;

        let local_size = 64;
        let global_size = 4;
        let num_elements = local_size * global_size;

        let kernel_data = [1u32, 3u32, 5u32, 7u32];
        let mut input_data = vec![0u32; num_elements + kernel_data.len() - 1];
        let mut output_data = vec![0u32; num_elements];

        let input_bytes = size_of_val(&input_data[..]) as gfx::DeviceSize;
        let kernel_bytes = size_of_val(&kernel_data[..]) as gfx::DeviceSize;
        let output_bytes = size_of_val(&output_data[..]) as gfx::DeviceSize;

        for (i, e) in input_data.iter_mut().enumerate() {
            *e = i as u32;
        }

        println!("- Creating buffers");
        let input_buffer = utils::UniqueBuffer::new(
            device,
            device
                .build_buffer()
                .label("Input buffer")
                .size(input_bytes)
                .usage(flags![gfx::BufferUsage::{Storage}])
                .build()
                .unwrap(),
        );
        let kernel_buffer = utils::UniqueBuffer::new(
            device,
            device
                .build_buffer()
                .label("Kernel buffer")
                .size(kernel_bytes)
                .usage(flags![gfx::BufferUsage::{Uniform}])
                .build()
                .unwrap(),
        );
        let output_buffer = utils::UniqueBuffer::new(
            device,
            device
                .build_buffer()
                .label("Output buffer")
                .size(output_bytes)
                .usage(flags![gfx::BufferUsage::{Storage}])
                .build()
                .unwrap(),
        );

        println!("- Computing the memory requirements for the heap");
        let valid_memory_types = [
            device.get_memory_req((&*input_buffer).into()).unwrap(),
            device.get_memory_req((&*kernel_buffer).into()).unwrap(),
            device.get_memory_req((&*output_buffer).into()).unwrap(),
        ].iter()
            .map(|r| r.memory_types)
            .fold(!0, |x, y| x & y);
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
            builder.prebind((&*input_buffer).into());
            builder.prebind((&*kernel_buffer).into());
            builder.prebind((&*output_buffer).into());
            builder.build().unwrap()
        };

        println!("- Retrieving pointers to the allocated buffer");
        let input_ptr = unsafe {
            let alloc = heap.bind((&*input_buffer).into()).unwrap().unwrap();
            let ptr = heap.as_ptr(&alloc).unwrap();
            from_raw_parts_mut(ptr as *mut u32, input_data.len())
        };
        let kernel_ptr = unsafe {
            let alloc = heap.bind((&*kernel_buffer).into()).unwrap().unwrap();
            let ptr = heap.as_ptr(&alloc).unwrap();
            from_raw_parts_mut(ptr as *mut u32, kernel_data.len())
        };
        let output_ptr = unsafe {
            let alloc = heap.bind((&*output_buffer).into()).unwrap().unwrap();
            let ptr = heap.as_ptr(&alloc).unwrap();
            from_raw_parts_mut(ptr as *mut u32, output_data.len())
        };
        println!(
            "  Input = {:p}, Kernel = {:p}, Output = {:p}",
            input_ptr, kernel_ptr, output_ptr
        );

        println!("- Storing the shader inputs");
        input_ptr.copy_from_slice(&input_data);
        kernel_ptr.copy_from_slice(&kernel_data);

        println!("- Creating a command queue");
        let queue = device
            .build_cmd_queue()
            .queue_family(qf)
            .label("Main queue")
            .build()
            .unwrap();

        println!("- Creating a library");
        let library = device.new_library(SPIRV_CONV.as_u32_slice()).unwrap();

        println!("- Creating an argument table signature");
        let arg_table_sig = {
            let mut builder = device.build_arg_table_sig();
            builder.arg(binding_redundant, gfx::ArgType::UniformBuffer);
            builder.arg(binding_input, gfx::ArgType::StorageBuffer);
            builder.arg(binding_output, gfx::ArgType::StorageBuffer);
            builder.arg(binding_param, gfx::ArgType::UniformBuffer);
            builder.build().unwrap()
        };

        println!("- Creating a root signature");
        let root_sig = device
            .build_root_sig()
            .arg_table(0, &arg_table_sig)
            .arg_table(1, &arg_table_sig)
            .build()
            .unwrap();

        println!("- Creating an argument pool");
        let mut arg_pool: Box<gfx::ArgPool> = device
            .build_arg_pool()
            .reserve_table_sig(2, &arg_table_sig)
            .build()
            .unwrap();

        println!("- Creating an argument table");
        // The first one is actually unused -- The intention is to check if
        // constant buffer alignment restriction is enforced in the Metal backend
        arg_pool.new_table(&arg_table_sig).unwrap().unwrap();
        let arg_table = arg_pool.new_table(&arg_table_sig).unwrap().unwrap();

        println!("- Writing the argument table");
        device
            .update_arg_table(
                &arg_table_sig,
                &arg_table,
                &[
                    (binding_redundant, 0, [(0..4, &*kernel_buffer)][..].into()),
                    (
                        binding_input,
                        0,
                        [(0..input_bytes, &*input_buffer)][..].into(),
                    ),
                    (
                        binding_output,
                        0,
                        [(0..output_bytes, &*output_buffer)][..].into(),
                    ),
                    (
                        binding_param,
                        0,
                        [(0..kernel_bytes, &*kernel_buffer)][..].into(),
                    ),
                ],
            )
            .unwrap();

        println!("- Creating a pipeline");
        let pipeline = device
            .build_compute_pipeline()
            .compute_shader(&library, "main")
            .root_sig(&root_sig)
            .label("Convolution pipeline")
            .build()
            .unwrap();

        println!("- Creating a command buffer");
        let mut buffer: Box<gfx::CmdBuffer> = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e: &mut gfx::ComputeCmdEncoder = buffer.encode_compute();
            e.use_resource(
                gfx::ResourceUsage::Read,
                &[(&*input_buffer).into(), (&*kernel_buffer).into()],
            );
            e.use_resource(gfx::ResourceUsage::Write, &[(&*output_buffer).into()]);
            e.begin_debug_group("Convolution");
            e.bind_pipeline(&pipeline);
            e.bind_arg_table(0, &[&arg_table]);
            e.bind_arg_table(1, &[&arg_table]);
            e.dispatch(&[global_size as u32]);
            e.end_debug_group();
        }
        buffer.host_barrier(
            flags![gfx::AccessType::{ComputeWrite}],
            &[(0..output_bytes, &*output_buffer)],
        );

        println!("- Installing a completion handler");
        let awaiter = utils::CmdBufferAwaiter::new(&mut *buffer);

        println!("- Commiting the command buffer");
        buffer.commit().unwrap();

        println!("- Flushing the command queue");
        queue.flush();

        println!("- Waiting for completion");
        awaiter.wait_until_completed();

        println!("- Reading back the result");
        output_data.copy_from_slice(output_ptr);

        let mut model_data = vec![0u32; num_elements];
        for (i, model) in model_data.iter_mut().enumerate() {
            let mut sum = 0;
            for (k, kern) in kernel_data.iter().enumerate() {
                sum += input_data[i + k] * kern;
            }
            *model = sum;
        }

        assert_eq!(output_data, model_data.as_slice());
    });
}
