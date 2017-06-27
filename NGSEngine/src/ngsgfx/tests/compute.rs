//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

extern crate ngsgfx as gfx;
extern crate cgmath;
#[macro_use]
extern crate include_data;

use gfx::core;
use gfx::prelude::*;

use cgmath::Vector3;

use std::{time, mem, ptr};

static SPIRV_NULL: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/compute_null.comp.spv"));
static SPIRV_CONV1: include_data::DataView =
    include_data!(concat!(env!("OUT_DIR"), "/compute_conv1.comp.spv"));

trait BackendDispatch {
    fn use_device<B: core::Backend>(self, device: B::Device);
}

#[cfg(target_os = "macos")]
fn try_device_metal<T: BackendDispatch>(d: T) -> Option<T> {
    use gfx::backends::metal::ll::NSObjectProtocol;
    let arp = gfx::backends::metal::ll::NSAutoreleasePool::alloc().init();
    let metal_device = gfx::backends::metal::ll::create_system_default_device();
    let device = gfx::backends::metal::imp::Device::new(metal_device);
    d.use_device::<gfx::backends::metal::Backend>(device);
    unsafe {
        arp.release();
    }
    None
}

#[cfg(not(target_os = "macos"))]
fn try_device_metal<T: BackendDispatch>(d: T) -> Option<T> {
    Some(d)
}

fn find_default_device<T: BackendDispatch>(d: T) {
    let t = Some(d).and_then(try_device_metal);
    if t.is_some() {
        panic!("no backend available -- cannot proceed");
    }
}

struct DeviceUtils<'a, B: core::Backend>(&'a B::Device);
struct ResultBuffer<'a, B: core::Backend, T: 'static>(&'a B::Device, B::Buffer, &'a mut [T]);

impl<'a, B: core::Backend, T: 'static> ResultBuffer<'a, B, T> {
    fn size(&self) -> usize {
        mem::size_of_val(self.2)
    }
    fn buffer(&self) -> &B::Buffer {
        &self.1
    }
    fn take(
        self,
        last_pipeline_stage: core::PipelineStageFlags,
        last_access_mask: core::AccessTypeFlags,
    ) -> &'a mut [T] {
        let device = self.0;

        let size = mem::size_of_val(self.2);
        let staging_buffer_desc = core::BufferDescription {
            usage: core::BufferUsage::TransferDestination.into(),
            size,
        };

        let factory = device.factory();
        let buffer = self.1;

        // Create a staging heap/buffer
        let staging_req: core::MemoryRequirements =
            factory.get_buffer_memory_requirements(&staging_buffer_desc);
        let mut staging_heap = factory
            .make_heap(&core::HeapDescription {
                size: staging_req.size,
                storage_mode: core::StorageMode::Shared,
            })
            .unwrap();

        let (mut staging_alloc, staging_buffer) = staging_heap
            .make_buffer(&staging_buffer_desc)
            .unwrap()
            .unwrap();

        staging_buffer.set_label(Some("staging buffer"));

        // Fill the buffer
        let queue = device.main_queue();
        let mut cb = queue.make_command_buffer().unwrap();
        cb.set_label(Some("staging CB"));
        cb.begin_encoding();
        cb.barrier(
            last_pipeline_stage,
            core::PipelineStage::Transfer.into(),
            &[
                core::Barrier::BufferMemoryBarrier {
                    buffer: &buffer,
                    source_access_mask: last_access_mask,
                    destination_access_mask: core::AccessType::TransferWrite.into(),
                    offset: 0,
                    len: size,
                },
            ],
        );
        cb.begin_blit_pass();
        cb.begin_debug_group(&core::DebugMarker::new("staging to buffer"));
        cb.copy_buffer(&buffer, 0, &staging_buffer, 0, size);
        cb.end_debug_group();
        cb.end_pass();
        cb.barrier(
            core::PipelineStage::Transfer.into(),
            core::PipelineStage::Host.into(),
            &[
                core::Barrier::BufferMemoryBarrier {
                    buffer: &staging_buffer,
                    source_access_mask: core::AccessType::TransferWrite.into(),
                    destination_access_mask: core::AccessType::HostWrite.into(),
                    offset: 0,
                    len: size,
                },
            ],
        );
        cb.end_encoding();

        queue.submit_commands(&[&cb], None).unwrap();

        assert_eq!(
            cb.wait_completion(time::Duration::from_secs(1)).unwrap(),
            true
        );

        {
            let map = staging_heap.map_memory(&mut staging_alloc);
            unsafe {
                ptr::copy(map.as_ptr() as *mut T, self.2.as_mut_ptr(), self.2.len());
            }
        }

        self.2
    }
}

impl<'a, B: core::Backend> DeviceUtils<'a, B> {
    fn make_result_buffer<T: 'static>(
        &self,
        data: &'a mut [T],
        usage: core::BufferUsageFlags,
    ) -> ResultBuffer<'a, B, T> {
        let device = self.0;

        let size = mem::size_of_val(data);
        let buffer_desc = core::BufferDescription { usage, size };

        let factory = device.factory();

        // Create a device heap/buffer
        let req: core::MemoryRequirements = factory.get_buffer_memory_requirements(&buffer_desc);
        let mut heap = factory
            .make_heap(&core::HeapDescription {
                size: req.size,
                storage_mode: core::StorageMode::Private,
            })
            .unwrap();

        let buffer = heap.make_buffer(&buffer_desc).unwrap().unwrap().1;
        ResultBuffer(self.0, buffer, data)
    }

    fn make_preinitialized_buffer<T>(
        &self,
        data: &[T],
        usage: core::BufferUsageFlags,
        first_pipeline_stage: core::PipelineStageFlags,
        first_access_mask: core::AccessTypeFlags,
    ) -> B::Buffer {
        let device = self.0;

        let size = mem::size_of_val(data);
        let staging_buffer_desc = core::BufferDescription {
            usage: core::BufferUsage::TransferSource.into(),
            size,
        };
        let buffer_desc = core::BufferDescription { usage, size };

        let factory = device.factory();

        // Create a staging heap/buffer
        let staging_req: core::MemoryRequirements =
            factory.get_buffer_memory_requirements(&staging_buffer_desc);
        let mut staging_heap = factory
            .make_heap(&core::HeapDescription {
                size: staging_req.size,
                storage_mode: core::StorageMode::Shared,
            })
            .unwrap();

        let (mut staging_alloc, staging_buffer) = staging_heap
            .make_buffer(&staging_buffer_desc)
            .unwrap()
            .unwrap();
        {
            let mut map = staging_heap.map_memory(&mut staging_alloc);
            unsafe {
                ptr::copy(data.as_ptr(), map.as_mut_ptr() as *mut T, data.len());
            }
        }

        // Create a device heap/buffer
        let req: core::MemoryRequirements = factory.get_buffer_memory_requirements(&buffer_desc);
        let mut heap = factory
            .make_heap(&core::HeapDescription {
                size: req.size,
                storage_mode: core::StorageMode::Private,
            })
            .unwrap();

        let buffer = heap.make_buffer(&buffer_desc).unwrap().unwrap().1;

        // Add debug labels
        buffer.set_label(Some("preinitialized buffer"));
        staging_buffer.set_label(Some("staging buffer"));

        // Fill the buffer
        let queue = device.main_queue();
        let mut cb = queue.make_command_buffer().unwrap();
        cb.set_label(Some("staging CB to buffer"));
        cb.begin_encoding();
        cb.barrier(
            core::PipelineStage::Host.into(),
            core::PipelineStage::Transfer.into(),
            &[
                core::Barrier::BufferMemoryBarrier {
                    buffer: &staging_buffer,
                    source_access_mask: core::AccessType::HostWrite.into(),
                    destination_access_mask: core::AccessType::TransferRead.into(),
                    offset: 0,
                    len: size,
                },
            ],
        );
        cb.begin_blit_pass();
        cb.begin_debug_group(&core::DebugMarker::new("staging to buffer"));
        cb.copy_buffer(&staging_buffer, 0, &buffer, 0, size);
        cb.end_debug_group();
        cb.end_pass();
        cb.barrier(
            core::PipelineStage::Transfer.into(),
            first_pipeline_stage,
            &[
                core::Barrier::BufferMemoryBarrier {
                    buffer: &buffer,
                    source_access_mask: core::AccessType::TransferWrite.into(),
                    destination_access_mask: first_access_mask,
                    offset: 0,
                    len: size,
                },
            ],
        );
        cb.end_encoding();

        queue.submit_commands(&[&cb], None).unwrap();

        assert_eq!(
            cb.wait_completion(time::Duration::from_secs(1)).unwrap(),
            true
        );

        // Phew! Done!
        buffer
    }
}

#[test]
fn simple() {
    find_default_device(SimpleTest);
}

struct SimpleTest;
impl BackendDispatch for SimpleTest {
    fn use_device<B: core::Backend>(self, device: B::Device) {
        let factory = device.factory();

        let shader_desc = core::ShaderModuleDescription { spirv_code: SPIRV_NULL.as_u32_slice() };
        let shader = factory.make_shader_module(&shader_desc).unwrap();

        let layout_desc = core::PipelineLayoutDescription { descriptor_set_layouts: &[] };
        let layout = factory.make_pipeline_layout(&layout_desc).unwrap();

        let pipeline_desc = core::ComputePipelineDescription {
            label: Some("test compute pipeline: null"),
            shader_stage: core::ShaderStageDescription {
                stage: core::ShaderStage::Compute,
                module: &shader,
                entry_point_name: "main",
            },
            pipeline_layout: &layout,
        };

        let pipeline = factory.make_compute_pipeline(&pipeline_desc).unwrap();

        let queue = device.main_queue();
        let mut cb = queue.make_command_buffer().unwrap();

        cb.begin_encoding();
        cb.begin_compute_pass();
        cb.bind_compute_pipeline(&pipeline);
        cb.dispatch(Vector3::new(1, 1, 1));
        cb.end_pass();
        cb.end_encoding();

        queue.submit_commands(&[&cb], None).unwrap();
        assert_eq!(
            cb.wait_completion(time::Duration::from_secs(1)).unwrap(),
            true
        );
    }
}

#[test]
fn conv1() {
    find_default_device(Conv1Test);
}

struct Conv1Test;
impl BackendDispatch for Conv1Test {
    fn use_device<B: core::Backend>(self, device: B::Device) {
        let binding_param = 0;
        let binding_input = 1;
        let binding_output = 2;

        let factory = device.factory();
        let device_utils = DeviceUtils::<B>(&device);

        let local_size = 64;
        let global_size = 1;
        let num_elements = local_size * global_size;

        let kernel_data = [1f32, 3f32, 5f32, 7f32];
        let mut input_data = vec![0f32; num_elements + kernel_data.len() - 1];
        let mut output_data = vec![0f32; num_elements];

        for (i, e) in input_data.iter_mut().enumerate() {
            *e = i as f32;
        }

        let input_buffer = device_utils.make_preinitialized_buffer(
            input_data.as_slice(),
            core::BufferUsage::StorageBuffer.into(),
            core::PipelineStage::ComputeShader.into(),
            core::AccessType::ShaderRead.into(),
        );

        let kernel_buffer = device_utils.make_preinitialized_buffer(
            &kernel_data,
            core::BufferUsage::StorageBuffer.into(),
            core::PipelineStage::ComputeShader.into(),
            core::AccessType::ShaderRead.into(),
        );

        let output_buffer = device_utils.make_result_buffer(
            output_data.as_mut_slice(),
            core::BufferUsage::StorageBuffer.into(),
        );

        let shader_desc = core::ShaderModuleDescription { spirv_code: SPIRV_CONV1.as_u32_slice() };
        let shader = factory.make_shader_module(&shader_desc).unwrap();

        let set_layout_desc = core::DescriptorSetLayoutDescription {
            bindings: &[
                core::DescriptorSetLayoutBinding {
                    location: binding_param,
                    descriptor_type: core::DescriptorType::StorageBuffer,
                    num_elements: 1,
                    stage_flags: core::ShaderStage::Compute.into(),
                    immutable_samplers: None,
                },
                core::DescriptorSetLayoutBinding {
                    location: binding_input,
                    descriptor_type: core::DescriptorType::StorageBuffer,
                    num_elements: 1,
                    stage_flags: core::ShaderStage::Compute.into(),
                    immutable_samplers: None,
                },
                core::DescriptorSetLayoutBinding {
                    location: binding_output,
                    descriptor_type: core::DescriptorType::StorageBuffer,
                    num_elements: 1,
                    stage_flags: core::ShaderStage::Compute.into(),
                    immutable_samplers: None,
                },
            ],
        };
        let set_layout = factory
            .make_descriptor_set_layout(&set_layout_desc)
            .unwrap();

        let layout_desc =
            core::PipelineLayoutDescription { descriptor_set_layouts: &[&set_layout] };
        let layout = factory.make_pipeline_layout(&layout_desc).unwrap();

        let mut desc_pool = factory
            .make_descriptor_pool(&core::DescriptorPoolDescription {
                max_num_sets: 1,
                pool_sizes: &[
                    core::DescriptorPoolSize {
                        descriptor_type: core::DescriptorType::StorageBuffer,
                        num_descriptors: 3,
                    },
                ],
                supports_deallocation: false,
            })
            .unwrap();
        let desc_set = desc_pool
            .make_descriptor_set(&core::DescriptorSetDescription { layout: &set_layout })
            .unwrap()
            .unwrap()
            .0;

        let pipeline_desc = core::ComputePipelineDescription {
            label: Some("test compute pipeline: null"),
            shader_stage: core::ShaderStageDescription {
                stage: core::ShaderStage::Compute,
                module: &shader,
                entry_point_name: "main",
            },
            pipeline_layout: &layout,
        };

        desc_set.update(
            &[
                core::WriteDescriptorSet {
                    start_binding: binding_param,
                    start_index: 0,
                    elements: core::WriteDescriptors::StorageBuffer(
                        &[
                            core::DescriptorBuffer {
                                buffer: &kernel_buffer,
                                offset: 0,
                                range: mem::size_of_val(&kernel_data),
                            },
                        ],
                    ),
                },
                core::WriteDescriptorSet {
                    start_binding: binding_input,
                    start_index: 0,
                    elements: core::WriteDescriptors::StorageBuffer(
                        &[
                            core::DescriptorBuffer {
                                buffer: &input_buffer,
                                offset: 0,
                                range: mem::size_of_val(input_data.as_slice()),
                            },
                        ],
                    ),
                },
                core::WriteDescriptorSet {
                    start_binding: binding_output,
                    start_index: 0,
                    elements: core::WriteDescriptors::StorageBuffer(
                        &[
                            core::DescriptorBuffer {
                                buffer: output_buffer.buffer(),
                                offset: 0,
                                range: output_buffer.size(),
                            },
                        ],
                    ),
                },
            ],
        );

        let pipeline = factory.make_compute_pipeline(&pipeline_desc).unwrap();

        let queue = device.main_queue();
        let mut cb = queue.make_command_buffer().unwrap();

        cb.begin_encoding();
        cb.begin_compute_pass();
        cb.bind_compute_pipeline(&pipeline);
        cb.dispatch(Vector3::new(global_size as u32, 1, 1));
        cb.end_pass();
        cb.end_encoding();

        queue.submit_commands(&[&cb], None).unwrap();
        assert_eq!(
            cb.wait_completion(time::Duration::from_secs(1)).unwrap(),
            true
        );

        let result = output_buffer.take(
            core::PipelineStage::ComputeShader.into(),
            core::AccessType::ShaderWrite.into(),
        );

        // TODO: check output value
        println!("{:?}", result);
    }
}
