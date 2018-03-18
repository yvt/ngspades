//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `Device` for Vulkan.
use std::sync::Arc;
use arrayvec::ArrayVec;

use ash::vk;
use ash::version::*;

use {base, AshDevice};
use {arg, buffer, cmd, heap, image, limits, pipeline, renderpass, sampler, shader, utils};
use common::Result;

/// Unsafe reference to a Vulkan device object that is internally held by
/// `Device`.
///
/// This type is `'static`, but the referent is only guaranteed to live as long
/// as the originating `Device`. It is the application's responsibility to
/// prevent premature release of `Device` (as required by ZanGFX's base
/// interface.)
#[derive(Debug, Clone, Copy)]
pub(super) struct DeviceRef(*const AshDevice, *const limits::DeviceCaps);

unsafe impl Sync for DeviceRef {}
unsafe impl Send for DeviceRef {}

impl DeviceRef {
    pub fn vk_device(&self) -> &AshDevice {
        unsafe { &*self.0 }
    }

    pub fn caps(&self) -> &limits::DeviceCaps {
        unsafe { &*self.1 }
    }
}

/// Implementation of `Device` for Vulkan.
pub struct Device {
    // These fields are boxed so they can be referenced by `DeviceRef`
    vk_device: Box<AshDevice>,
    caps: Box<limits::DeviceCaps>,
    queue_pool: Arc<cmd::queue::QueuePool>,
}

zangfx_impl_object! { Device: base::Device, ::Debug }

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

        Ok(Self {
            vk_device: Box::new(vk_device),
            caps: Box::new(caps),
            queue_pool: Arc::new(queue_pool),
        })
    }

    pub fn vk_device(&self) -> &AshDevice {
        &self.vk_device
    }

    /// Construct a `DeviceRef` pointing this `Device`.
    pub(super) unsafe fn new_device_ref(&self) -> DeviceRef {
        DeviceRef(&*self.vk_device, &*self.caps)
    }
}

use std::fmt;
impl ::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Device").finish()
    }
}

impl base::Device for Device {
    fn caps(&self) -> &base::DeviceCaps {
        &*self.caps
    }

    fn build_cmd_queue(&self) -> Box<base::CmdQueueBuilder> {
        unsafe {
            Box::new(cmd::queue::CmdQueueBuilder::new(
                self.new_device_ref(),
                self.queue_pool.clone(),
            ))
        }
    }

    fn build_semaphore(&self) -> Box<base::SemaphoreBuilder> {
        unimplemented!()
    }

    fn build_dynamic_heap(&self) -> Box<base::DynamicHeapBuilder> {
        unsafe { Box::new(heap::DynamicHeapBuilder::new(self.new_device_ref())) }
    }

    fn build_dedicated_heap(&self) -> Box<base::DedicatedHeapBuilder> {
        unsafe { Box::new(heap::DedicatedHeapBuilder::new(self.new_device_ref())) }
    }

    fn build_barrier(&self) -> Box<base::BarrierBuilder> {
        Box::new(cmd::barrier::BarrierBuilder::new())
    }

    fn build_image(&self) -> Box<base::ImageBuilder> {
        unsafe { Box::new(image::ImageBuilder::new(self.new_device_ref())) }
    }

    fn build_buffer(&self) -> Box<base::BufferBuilder> {
        unsafe { Box::new(buffer::BufferBuilder::new(self.new_device_ref())) }
    }

    fn build_sampler(&self) -> Box<base::SamplerBuilder> {
        unsafe { Box::new(sampler::SamplerBuilder::new(self.new_device_ref())) }
    }

    fn build_image_view(&self) -> Box<base::ImageViewBuilder> {
        unsafe { Box::new(image::ImageViewBuilder::new(self.new_device_ref())) }
    }

    fn build_library(&self) -> Box<base::LibraryBuilder> {
        unsafe { Box::new(shader::LibraryBuilder::new(self.new_device_ref())) }
    }

    fn build_arg_table_sig(&self) -> Box<base::ArgTableSigBuilder> {
        unsafe { Box::new(arg::layout::ArgTableSigBuilder::new(self.new_device_ref())) }
    }

    fn build_root_sig(&self) -> Box<base::RootSigBuilder> {
        unsafe { Box::new(arg::layout::RootSigBuilder::new(self.new_device_ref())) }
    }

    fn build_arg_pool(&self) -> Box<base::ArgPoolBuilder> {
        unsafe { Box::new(arg::pool::ArgPoolBuilder::new(self.new_device_ref())) }
    }

    fn build_render_pass(&self) -> Box<base::RenderPassBuilder> {
        unsafe { Box::new(renderpass::RenderPassBuilder::new(self.new_device_ref())) }
    }

    fn build_render_target_table(&self) -> Box<base::RenderTargetTableBuilder> {
        unimplemented!()
    }

    fn build_render_pipeline(&self) -> Box<base::RenderPipelineBuilder> {
        unsafe { Box::new(pipeline::RenderPipelineBuilder::new(self.new_device_ref())) }
    }

    fn build_compute_pipeline(&self) -> Box<base::ComputePipelineBuilder> {
        unsafe { Box::new(pipeline::ComputePipelineBuilder::new(self.new_device_ref())) }
    }

    fn destroy_image(&self, obj: &base::Image) -> Result<()> {
        let our_image: &image::Image = obj.downcast_ref().expect("bad image type");
        unsafe {
            our_image.destroy(self.vk_device());
        }
        Ok(())
    }

    fn destroy_buffer(&self, obj: &base::Buffer) -> Result<()> {
        let our_buffer: &buffer::Buffer = obj.downcast_ref().expect("bad buffer type");
        unsafe {
            our_buffer.destroy(self.vk_device());
        }
        Ok(())
    }

    fn destroy_sampler(&self, obj: &base::Sampler) -> Result<()> {
        let our_sampler: &sampler::Sampler = obj.downcast_ref().expect("bad sampler type");
        unsafe {
            our_sampler.destroy(self.vk_device());
        }
        Ok(())
    }

    fn destroy_image_view(&self, obj: &base::ImageView) -> Result<()> {
        let our_image_view: &image::ImageView = obj.downcast_ref().expect("bad image view type");
        unsafe {
            our_image_view.destroy(self.vk_device());
        }
        Ok(())
    }

    fn get_memory_req(&self, obj: base::ResourceRef) -> Result<base::MemoryReq> {
        utils::get_memory_req(self.vk_device(), obj)
    }

    fn update_arg_tables(
        &self,
        arg_table_sig: &base::ArgTableSig,
        updates: &[(&base::ArgTable, &[base::ArgUpdateSet])],
    ) -> Result<()> {
        let vk_device = self.vk_device();
        let table_sig: &arg::layout::ArgTableSig = arg_table_sig
            .downcast_ref()
            .expect("bad argument table signature type");

        let mut writes: ArrayVec<[vk::WriteDescriptorSet; 256]> = ArrayVec::new();
        let mut write_images: ArrayVec<[vk::DescriptorImageInfo; 256]> = ArrayVec::new();
        let mut write_buffers: ArrayVec<[vk::DescriptorBufferInfo; 256]> = ArrayVec::new();

        macro_rules! flush {
            () => ({
                unsafe {
                    vk_device.update_descriptor_sets(writes.as_slice(), &[]);
                }
                writes.clear();
                write_images.clear();
                write_buffers.clear();
            })
        }

        fn vec_end_ptr<T>(v: &[T]) -> *const T {
            v.as_ptr().wrapping_offset(v.len() as isize)
        }

        for &(table, update_sets) in updates.iter() {
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
                        base::ArgSlice::ImageView(views) => {
                            while !write_images.is_full() && i < views.len() {
                                let view = views[i];
                                let view: &image::ImageView =
                                    view.downcast_ref().expect("bad image view type");

                                write_images.push(vk::DescriptorImageInfo {
                                    sampler: vk::Sampler::null(),
                                    image_view: view.vk_image_view(),
                                    image_layout: view.meta().image_layout(),
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
