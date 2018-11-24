//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::{utils, TestDriver};
use flags_macro::flags;
use include_data::include_data;
use std::mem::size_of_val;
use volatile_view::prelude::*;
use zangfx_base as gfx;
use zangfx_base::prelude::*;
use zangfx_utils::prelude::*;

static SPIRV_CONV: ::include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/compute_conv1.comp.spv"));

/// Performs a convolution using a compute shader. Parameters are passed using
/// the normal dispatch command.
pub fn compute_conv1_direct<T: TestDriver>(driver: T) {
    compute_conv1_common(driver, true);
}

/// Performs a convolution using a compute shader. Parameters are passed using
/// the indirect dispatch command.
pub fn compute_conv1_indirect<T: TestDriver>(driver: T) {
    compute_conv1_common(driver, false);
}

/// Performs a convolution using a compute shader.
fn compute_conv1_common<T: TestDriver>(driver: T, direct: bool) {
    driver.for_each_compute_queue(&mut |device, qf| {
        let binding_redundant = 0; // unused -- evoke possible issue in arg table handling
        let binding_param = 1;
        let binding_input = 2;
        let binding_output = 3;

        let local_size = 64;
        let global_size = 4;
        let num_elements = local_size * global_size;

        let kernel_data = [[1u32; 4], [3u32; 4], [5u32; 4], [7u32; 4]];
        let mut input_data = vec![0u32; num_elements + kernel_data.len() - 1];
        let mut output_data = vec![0u32; num_elements];
        let indirect_data = [global_size as u32, 1, 1];

        let input_bytes = size_of_val(&input_data[..]) as gfx::DeviceSize;
        let kernel_bytes = size_of_val(&kernel_data[..]) as gfx::DeviceSize;
        let output_bytes = size_of_val(&output_data[..]) as gfx::DeviceSize;
        let indirect_bytes = size_of_val(&indirect_data[..]) as gfx::DeviceSize;

        for (i, e) in input_data.iter_mut().enumerate() {
            *e = i as u32;
        }

        println!("- Creating a command queue");
        let queue = device
            .build_cmd_queue()
            .queue_family(qf)
            .label("Main queue")
            .build()
            .unwrap();

        println!("- Creating buffers");
        let input_buffer = device
            .build_buffer()
            .label("Input buffer")
            .size(input_bytes)
            .usage(gfx::BufferUsageFlags::Storage)
            .queue(&queue)
            .build()
            .unwrap();
        let kernel_buffer = device
            .build_buffer()
            .label("Kernel buffer")
            .size(kernel_bytes)
            .usage(gfx::BufferUsageFlags::Uniform)
            .queue(&queue)
            .build()
            .unwrap();
        let output_buffer = device
            .build_buffer()
            .label("Output buffer")
            .size(output_bytes)
            .usage(gfx::BufferUsageFlags::Storage)
            .queue(&queue)
            .build()
            .unwrap();
        let indirect_buffer = device
            .build_buffer()
            .label("Indirect argument buffer")
            .size(indirect_bytes)
            .usage(gfx::BufferUsageFlags::IndirectDraw)
            .queue(&queue)
            .build()
            .unwrap();

        println!("- Computing the memory requirements for the heap");
        let valid_memory_types = [
            &input_buffer,
            &kernel_buffer,
            &output_buffer,
            &indirect_buffer,
        ]
        .iter()
        .map(|r| r.get_memory_req().unwrap().memory_types)
        .fold(!0, |x, y| x & y);
        let memory_type = utils::choose_memory_type(
            device,
            valid_memory_types,
            flags![gfx::MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
            flags![gfx::MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
        );
        println!("  Memory Type = {}", memory_type);

        println!("- Allocating memory");
        let heap = device.global_heap(memory_type);
        heap.bind((&input_buffer).into()).unwrap();
        heap.bind((&kernel_buffer).into()).unwrap();
        heap.bind((&output_buffer).into()).unwrap();
        heap.bind((&indirect_buffer).into()).unwrap();

        println!("- Retrieving pointers to the allocated buffer");
        let input_view = input_buffer.as_volatile().unwrap();
        let kernel_view = kernel_buffer.as_volatile().unwrap();
        let output_view = output_buffer.as_volatile().unwrap();
        let indirect_view = indirect_buffer.as_volatile().unwrap();
        println!(
            "  Input = {:p}, Kernel = {:p}, Output = {:p}, Indirect = {:p}",
            input_view.as_ptr(),
            kernel_view.as_ptr(),
            output_view.as_ptr(),
            indirect_view.as_ptr()
        );

        println!("- Storing the shader inputs");
        input_view.copy_from_slice(&input_data);
        kernel_view.copy_from_slice(&kernel_data);
        indirect_view.copy_from_slice(&indirect_data);

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
        let arg_pool: gfx::ArgPoolRef = device
            .build_arg_pool()
            .reserve_table_sig(2, &arg_table_sig)
            .queue(&queue)
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
                &arg_pool,
                &arg_table,
                &[
                    (binding_redundant, 0, [(0..4, &kernel_buffer)][..].into()),
                    (
                        binding_input,
                        0,
                        [(0..input_bytes, &input_buffer)][..].into(),
                    ),
                    (
                        binding_output,
                        0,
                        [(0..output_bytes, &output_buffer)][..].into(),
                    ),
                    (
                        binding_param,
                        0,
                        [(0..kernel_bytes, &kernel_buffer)][..].into(),
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
        let mut buffer = queue.new_cmd_buffer().unwrap();

        println!("- Encoding the command buffer");
        {
            let e: &mut dyn gfx::ComputeCmdEncoder = buffer.encode_compute();
            e.use_resource_read(&[&input_buffer, &kernel_buffer][..]);
            e.use_resource_read_write(&output_buffer);
            e.begin_debug_group("Convolution");
            e.bind_pipeline(&pipeline);
            e.bind_arg_table(0, &[(&arg_pool, &arg_table)]);
            e.bind_arg_table(1, &[(&arg_pool, &arg_table)]);
            if direct {
                e.dispatch(&[global_size as u32]);
            } else {
                e.dispatch_indirect(&indirect_buffer, 0);
            }
            e.end_debug_group();
        }
        buffer.host_barrier(
            gfx::AccessTypeFlags::ComputeWrite,
            &[(0..output_bytes, &output_buffer)],
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
        output_view.copy_to_slice(&mut output_data);

        let mut model_data = vec![0u32; num_elements];
        for (i, model) in model_data.iter_mut().enumerate() {
            let mut sum = 0;
            for (k, kern) in kernel_data.iter().enumerate() {
                sum += input_data[i + k] * kern[0];
            }
            *model = sum;
        }

        assert_eq!(output_data, model_data.as_slice());
    });
}
