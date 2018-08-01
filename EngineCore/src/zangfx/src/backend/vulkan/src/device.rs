//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Device` for Vulkan.
use arrayvec::ArrayVec;
use parking_lot::RwLock;
use std::sync::Arc;

use ash::version::*;
use ash::vk;

use crate::AshDevice;
use crate::{
    arg, buffer, cmd, heap, image, limits, pipeline, renderpass, resstate, sampler, shader,
};
use zangfx_base::Result;
use zangfx_base::{self as base, interfaces, vtable_for, zangfx_impl_object};

crate struct DeviceInfo {
    vk_device: AshDevice,
    caps: limits::DeviceCaps,
    sampler_pool: sampler::SamplerPool,

    /// The default queue identifier (for resource state tracking) used during
    /// object creation.
    default_resstate_queue: RwLock<Option<resstate::QueueId>>,
}

crate type DeviceRef = Arc<DeviceInfo>;

impl DeviceInfo {
    crate fn vk_device(&self) -> &AshDevice {
        &self.vk_device
    }

    crate fn caps(&self) -> &limits::DeviceCaps {
        &self.caps
    }

    crate fn sampler_pool(&self) -> &sampler::SamplerPool {
        &self.sampler_pool
    }

    /// Get the default `resstate::QueueId`. Returns a dummy value if none is set.
    crate fn default_resstate_queue(&self) -> resstate::QueueId {
        self.default_resstate_queue
            .read()
            .unwrap_or_else(resstate::QueueId::dummy_value)
    }

    crate fn set_default_resstate_queue(&self, queue_id: resstate::QueueId) {
        *self.default_resstate_queue.write() = Some(queue_id);
    }

    crate fn set_default_resstate_queue_if_missing(&self, queue_id: resstate::QueueId) {
        let mut cell = self.default_resstate_queue.write();
        if cell.is_none() {
            *cell = Some(queue_id);
        }
    }
}

impl Drop for DeviceInfo {
    fn drop(&mut self) {
        self.sampler_pool.destroy(&self.vk_device);
    }
}

/// Implementation of `Device` for Vulkan.
#[derive(Debug)]
pub struct Device {
    device_ref: DeviceRef,
    queue_pool: Arc<cmd::queue::QueuePool>,
    global_heaps: Vec<base::HeapRef>,
}

zangfx_impl_object! { Device: dyn base::Device, dyn (crate::Debug) }

unsafe impl Send for Device {}
unsafe impl Sync for Device {}

impl Device {
    /// Construct a `Device` using a given Vulkan device object and
    /// backend configurations;
    ///
    /// `Device` does not destroy the given `AshDevice` automatically when
    /// dropped.
    ///
    /// Fails and returns `Err(_)` if the configuration fails validation.
    pub unsafe fn new(
        vk_device: AshDevice,
        info: limits::DeviceInfo,
        config: limits::DeviceConfig,
    ) -> Result<Self> {
        let caps = limits::DeviceCaps::new(info, config)?;
        let queue_pool = cmd::queue::QueuePool::new(&caps.config);
        let sampler_pool = sampler::SamplerPool::new();

        let device_ref = Arc::new(DeviceInfo {
            vk_device,
            caps,
            sampler_pool,
            default_resstate_queue: RwLock::new(None),
        });

        let global_heaps = (device_ref.caps.config.heap_strategies)
            .iter()
            .enumerate()
            .map(|(i, heap_strategy)| {
                let global_heap =
                    heap::GlobalHeap::new(device_ref.clone(), heap_strategy.unwrap(), i as _);
                Arc::new(global_heap) as base::HeapRef
            })
            .collect();

        Ok(Self {
            device_ref,
            queue_pool: Arc::new(queue_pool),
            global_heaps,
        })
    }

    pub fn vk_device(&self) -> &AshDevice {
        &self.device_ref.vk_device()
    }

    /// Set the default queue to be used during object creation.
    ///
    /// See [the crate documentation](../index.html) for more details about
    /// inter-queue operations.
    pub fn set_default_queue(&self, queue: &cmd::queue::CmdQueue) {
        self.device_ref
            .set_default_resstate_queue(queue.resstate_queue_id());
    }

    crate fn device_ref(&self) -> &DeviceRef {
        &self.device_ref
    }
}

use std::fmt;
impl fmt::Debug for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceInfo")
            .field("vk_device", &())
            .field("caps", &self.caps)
            .finish()
    }
}

impl base::Device for Device {
    fn caps(&self) -> &dyn base::DeviceCaps {
        self.device_ref.caps()
    }

    fn global_heap(&self, memory_type: base::MemoryType) -> &base::HeapRef {
        self.global_heaps
            .get(memory_type as usize)
            .expect("bad memory type")
    }

    fn build_cmd_queue(&self) -> base::CmdQueueBuilderRef {
        unsafe {
            Box::new(cmd::queue::CmdQueueBuilder::new(
                self.device_ref().clone(),
                self.queue_pool.clone(),
            ))
        }
    }

    fn build_semaphore(&self) -> base::SemaphoreBuilderRef {
        Box::new(cmd::semaphore::SemaphoreBuilder::new(
            self.device_ref().clone(),
        ))
    }

    fn build_dynamic_heap(&self) -> base::DynamicHeapBuilderRef {
        Box::new(heap::DynamicHeapBuilder::new(self.device_ref().clone()))
    }

    fn build_dedicated_heap(&self) -> base::DedicatedHeapBuilderRef {
        Box::new(heap::DedicatedHeapBuilder::new(self.device_ref().clone()))
    }

    fn build_image(&self) -> base::ImageBuilderRef {
        Box::new(image::ImageBuilder::new(self.device_ref().clone()))
    }

    fn build_buffer(&self) -> base::BufferBuilderRef {
        Box::new(buffer::BufferBuilder::new(self.device_ref().clone()))
    }

    fn build_sampler(&self) -> base::SamplerBuilderRef {
        Box::new(sampler::SamplerBuilder::new(self.device_ref().clone()))
    }

    fn build_library(&self) -> base::LibraryBuilderRef {
        Box::new(shader::LibraryBuilder::new(self.device_ref().clone()))
    }

    fn build_arg_table_sig(&self) -> base::ArgTableSigBuilderRef {
        Box::new(arg::layout::ArgTableSigBuilder::new(
            self.device_ref().clone(),
        ))
    }

    fn build_root_sig(&self) -> base::RootSigBuilderRef {
        Box::new(arg::layout::RootSigBuilder::new(self.device_ref().clone()))
    }

    fn build_arg_pool(&self) -> base::ArgPoolBuilderRef {
        Box::new(arg::pool::ArgPoolBuilder::new(self.device_ref().clone()))
    }

    fn build_render_pass(&self) -> base::RenderPassBuilderRef {
        Box::new(renderpass::RenderPassBuilder::new(
            self.device_ref().clone(),
        ))
    }

    fn build_render_target_table(&self) -> base::RenderTargetTableBuilderRef {
        Box::new(renderpass::RenderTargetTableBuilder::new(
            self.device_ref().clone(),
        ))
    }

    fn build_render_pipeline(&self) -> base::RenderPipelineBuilderRef {
        Box::new(pipeline::RenderPipelineBuilder::new(
            self.device_ref().clone(),
        ))
    }

    fn build_compute_pipeline(&self) -> base::ComputePipelineBuilderRef {
        Box::new(pipeline::ComputePipelineBuilder::new(
            self.device_ref().clone(),
        ))
    }

    fn update_arg_tables(
        &self,
        arg_table_sig: &base::ArgTableSigRef,
        updates: &[(
            (&base::ArgPoolRef, &base::ArgTableRef),
            &[base::ArgUpdateSet<'_>],
        )],
    ) -> Result<()> {
        let vk_device = self.vk_device();
        let table_sig: &arg::layout::ArgTableSig = arg_table_sig
            .downcast_ref()
            .expect("bad argument table signature type");

        let mut writes: ArrayVec<[vk::WriteDescriptorSet; 256]> = ArrayVec::new();
        let mut write_images: ArrayVec<[vk::DescriptorImageInfo; 256]> = ArrayVec::new();
        let mut write_buffers: ArrayVec<[vk::DescriptorBufferInfo; 256]> = ArrayVec::new();

        macro_rules! flush {
            () => {{
                unsafe {
                    vk_device.update_descriptor_sets(writes.as_slice(), &[]);
                }
                writes.clear();
                write_images.clear();
                write_buffers.clear();
            }};
        }

        fn vec_end_ptr<T>(v: &[T]) -> *const T {
            v.as_ptr().wrapping_offset(v.len() as isize)
        }

        for &((_pool, table), update_sets) in updates.iter() {
            let table: &arg::pool::ArgTable =
                table.downcast_ref().expect("bad argument table type");
            for &(arg_i, mut array_i, objs) in update_sets.iter() {
                if objs.len() == 0 {
                    continue;
                }

                let descriptor_type = table_sig.desc_type(arg_i).expect("invalid argument index");

                let mut i = 0;
                while i < objs.len() {
                    if writes.is_full() || write_images.is_full() || write_buffers.is_full() {
                        flush!();
                    }
                    let mut write = vk::WriteDescriptorSet {
                        s_type: vk::StructureType::WriteDescriptorSet,
                        p_next: ::null(),
                        dst_set: table.vk_descriptor_set(),
                        dst_binding: arg_i as u32,
                        dst_array_element: array_i as u32,
                        descriptor_count: 0, // set later
                        descriptor_type,
                        p_image_info: vec_end_ptr(&write_images),
                        p_buffer_info: vec_end_ptr(&write_buffers),
                        p_texel_buffer_view: ::null(),
                    };
                    let mut descriptor_count = 0;
                    match objs {
                        base::ArgSlice::Buffer(buffers) => {
                            while !write_buffers.is_full() && i < buffers.len() {
                                let (ref range, ref buffer) = buffers[i];
                                let buffer: &buffer::Buffer =
                                    buffer.downcast_ref().expect("bad buffer type");

                                write_buffers.push(vk::DescriptorBufferInfo {
                                    buffer: buffer.vk_buffer(),
                                    offset: range.start,
                                    range: range.end - range.start,
                                });
                                i += 1;
                                descriptor_count += 1;
                            }
                        }
                        base::ArgSlice::Image(images) => {
                            while !write_images.is_full() && i < images.len() {
                                let image = images[i];
                                let image: &image::Image =
                                    image.downcast_ref().expect("bad image type");

                                write_images.push(vk::DescriptorImageInfo {
                                    sampler: vk::Sampler::null(),
                                    image_view: image.vk_image_view(),
                                    image_layout: image.translate_layout(base::ImageLayout::Shader),
                                });
                                i += 1;
                                descriptor_count += 1;
                            }
                        }
                        base::ArgSlice::Sampler(samplers) => {
                            while !write_images.is_full() && i < samplers.len() {
                                let sampler = samplers[i];
                                let sampler: &sampler::Sampler =
                                    sampler.downcast_ref().expect("bad sampler type");

                                write_images.push(vk::DescriptorImageInfo {
                                    sampler: sampler.vk_sampler(),
                                    image_view: vk::ImageView::null(),
                                    image_layout: vk::ImageLayout::Undefined,
                                });
                                i += 1;
                                descriptor_count += 1;
                            }
                        }
                    };
                    write.descriptor_count = descriptor_count;
                    writes.push(write);
                }
            }
        }

        if writes.len() > 0 {
            flush!();
        }
        Ok(())
    }
}
