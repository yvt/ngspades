//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::version::*;
use ash::vk;
use flags_macro::flags;
use std::ops::Range;

use zangfx_base as base;
use zangfx_common::IntoWithPad;

use crate::buffer::Buffer;
use crate::image::{Image, ImageStateAddresser};
use crate::utils::{translate_image_aspect, translate_image_subresource_range};

use super::enc::ImageUnitOp;
use super::{CmdBufferData, PassImageBarrier};

impl CmdBufferData {
    /// Encode necessary image layout transitions to use an image in a following
    /// copy command. Furthermore, add a given image to the reference table.
    ///
    /// This method must be used in place of `use_image_for_pass`
    /// for copy commands. For other commands, `use_image_for_pass` must be
    /// used.
    fn use_image_for_copy(
        &mut self,
        layout: vk::ImageLayout,
        access: vk::AccessFlags,
        image: &Image,
        range: &base::ImageLayerRange,
    ) {
        if image.translate_layout(base::ImageLayout::CopyRead) == vk::ImageLayout::GENERAL {
            // Per-command tracking is not necessary if the layouts for
            // `CopyRead` and `CopyWrite` are identical
            return self.use_image_for_pass(
                layout,
                layout,
                flags![base::AccessTypeFlags::{CopyRead | CopyWrite}],
                image,
            );
        }

        let vk_cmd_buffer = self.vk_cmd_buffer();

        let addresser = ImageStateAddresser::from_image(image);

        let (image_index, op) = self.ref_table.insert_image(image);

        let current_pass_i = self.passes.len() - 1;
        let current_pass = self.passes.last_mut().unwrap();

        let ref mut vk_image_barriers = self.temp.vk_image_barriers;
        vk_image_barriers.clear();

        // For each state-tracking unit...
        for i in addresser.indices_for_image_and_layer_range(image, range) {
            if let Some(ref mut unit_op) = op.units[i] {
                let (last_pass_i, image_barrier_i) = unit_op.last_pass;
                if last_pass_i == current_pass_i {
                    // This state-tracking unit has been accessed before by
                    // the same pass.
                    if unit_op.layout == layout {
                        continue;
                    }

                    current_pass.image_barriers[image_barrier_i].final_layout = layout;
                    unit_op.layout = layout;

                    vk_image_barriers.push(vk::ImageMemoryBarrier {
                        s_type: vk::StructureType::IMAGE_MEMORY_BARRIER,
                        p_next: crate::null(),
                        // Applications must insert `barrier` manually
                        src_access_mask: vk::AccessFlags::empty(),
                        dst_access_mask: access,
                        old_layout: unit_op.layout,
                        new_layout: layout,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        image: image.vk_image(),
                        subresource_range: translate_image_subresource_range(
                            &addresser.subrange_for_index(i).into(),
                            image.aspects(),
                        ),
                    });

                    continue;
                }
            }

            let image_barrier_i = current_pass.image_barriers.len();

            current_pass.image_barriers.push(PassImageBarrier {
                image_index,
                unit_index: i,
                initial_layout: layout,
                final_layout: layout,
                access: flags![base::AccessTypeFlags::{CopyRead | CopyWrite}],
            });

            op.units[i] = Some(ImageUnitOp {
                layout,
                last_pass: (current_pass_i, image_barrier_i),
            });
        }

        if vk_image_barriers.len() > 0 {
            let vk_device = self.device.vk_device();
            unsafe {
                vk_device.cmd_pipeline_barrier(
                    vk_cmd_buffer,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &vk_image_barriers,
                );
            }
        }

        // We don't need `VkImageView` for copy commands, so don't call
        // `insert_image_view` here
    }
}

impl base::CopyCmdEncoder for CmdBufferData {
    fn fill_buffer(&mut self, buffer: &base::BufferRef, range: Range<base::DeviceSize>, value: u8) {
        if range.start >= range.end {
            return;
        }
        let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
        let vk_device = self.device.vk_device();

        self.ref_table.insert_buffer(my_buffer);

        let data = (value as u32) * 0x1010101;

        unsafe {
            vk_device.cmd_fill_buffer(
                self.vk_cmd_buffer(),
                my_buffer.vk_buffer(),
                range.start,
                range.end - range.start,
                data,
            );
        }
    }

    fn copy_buffer(
        &mut self,
        src: &base::BufferRef,
        src_offset: base::DeviceSize,
        dst: &base::BufferRef,
        dst_offset: base::DeviceSize,
        size: base::DeviceSize,
    ) {
        let my_src: &Buffer = src.downcast_ref().expect("bad buffer type");
        let my_dst: &Buffer = dst.downcast_ref().expect("bad buffer type");
        let vk_device = self.device.vk_device();

        self.ref_table.insert_buffer(my_src);
        self.ref_table.insert_buffer(my_dst);

        unsafe {
            vk_device.cmd_copy_buffer(
                self.vk_cmd_buffer(),
                my_src.vk_buffer(),
                my_dst.vk_buffer(),
                &[vk::BufferCopy {
                    src_offset,
                    dst_offset,
                    size,
                }],
            );
        }
    }

    // TODO: automatic image layout transitions

    fn copy_buffer_to_image(
        &mut self,
        src: &base::BufferRef,
        src_range: &base::BufferImageRange,
        dst: &base::ImageRef,
        dst_aspect: base::ImageAspect,
        dst_range: &base::ImageLayerRange,
        dst_origin: &[u32],
        size: &[u32],
    ) {
        let my_src: &Buffer = src.downcast_ref().expect("bad source buffer type");
        let my_dst: &Image = dst.downcast_ref().expect("bad destination image type");

        self.ref_table.insert_buffer(my_src);
        self.use_image_for_copy(
            my_dst.translate_layout(base::ImageLayout::CopyWrite),
            vk::AccessFlags::TRANSFER_WRITE,
            my_dst,
            dst_range,
        );

        let dst_origin: [u32; 3] = dst_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        let vk_device = self.device.vk_device();

        unsafe {
            vk_device.cmd_copy_buffer_to_image(
                self.vk_cmd_buffer(),
                my_src.vk_buffer(),
                my_dst.vk_image(),
                my_dst.translate_layout(base::ImageLayout::CopyWrite),
                &[vk::BufferImageCopy {
                    buffer_offset: src_range.offset,
                    buffer_row_length: src_range.row_stride as u32,
                    buffer_image_height: src_range.plane_stride as u32,
                    image_subresource: my_dst.resolve_vk_subresource_layers(
                        dst_range,
                        translate_image_aspect(dst_aspect),
                    ),
                    image_offset: vk::Offset3D {
                        x: dst_origin[0] as i32,
                        y: dst_origin[1] as i32,
                        z: dst_origin[2] as i32,
                    },
                    image_extent: vk::Extent3D {
                        width: size[0],
                        height: size[1],
                        depth: size[2],
                    },
                }],
            );
        }
    }

    fn copy_image_to_buffer(
        &mut self,
        src: &base::ImageRef,
        src_aspect: base::ImageAspect,
        src_range: &base::ImageLayerRange,
        src_origin: &[u32],
        dst: &base::BufferRef,
        dst_range: &base::BufferImageRange,
        size: &[u32],
    ) {
        let my_src: &Image = src.downcast_ref().expect("bad source image type");
        let my_dst: &Buffer = dst.downcast_ref().expect("bad destination buffer type");

        self.ref_table.insert_buffer(my_dst);
        self.use_image_for_copy(
            my_src.translate_layout(base::ImageLayout::CopyRead),
            vk::AccessFlags::TRANSFER_READ,
            my_src,
            src_range,
        );

        let src_origin: [u32; 3] = src_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        let vk_device = self.device.vk_device();

        unsafe {
            vk_device.fp_v1_0().cmd_copy_image_to_buffer(
                self.vk_cmd_buffer(),
                my_src.vk_image(),
                my_src.translate_layout(base::ImageLayout::CopyRead),
                my_dst.vk_buffer(),
                1,
                &vk::BufferImageCopy {
                    buffer_offset: dst_range.offset,
                    buffer_row_length: dst_range.row_stride as u32,
                    buffer_image_height: dst_range.plane_stride as u32,
                    image_subresource: my_src.resolve_vk_subresource_layers(
                        src_range,
                        translate_image_aspect(src_aspect),
                    ),
                    image_offset: vk::Offset3D {
                        x: src_origin[0] as i32,
                        y: src_origin[1] as i32,
                        z: src_origin[2] as i32,
                    },
                    image_extent: vk::Extent3D {
                        width: size[0],
                        height: size[1],
                        depth: size[2],
                    },
                },
            );
        }
    }

    fn copy_image(
        &mut self,
        src: &base::ImageRef,
        src_range: &base::ImageLayerRange,
        src_origin: &[u32],
        dst: &base::ImageRef,
        dst_range: &base::ImageLayerRange,
        dst_origin: &[u32],
        size: &[u32],
    ) {
        let my_src: &Image = src.downcast_ref().expect("bad source image type");
        let my_dst: &Image = dst.downcast_ref().expect("bad destination image type");

        let mut src_layout = my_src.translate_layout(base::ImageLayout::CopyRead);
        let mut dst_layout = my_dst.translate_layout(base::ImageLayout::CopyWrite);

        let addresser = ImageStateAddresser::from_image(my_src);

        if my_src.vk_image() == my_dst.vk_image()
            && src_layout != dst_layout
            && addresser.layer_range_intersects(my_src, src_range, my_dst, dst_range)
        {
            // The intersection of the state-tracking unit sets of the source
            // and destination is not empty.
            // In this case, the `Generic` layout, which can be used both for
            // reading and writing, must be used for entire the affected units.
            src_layout = vk::ImageLayout::GENERAL;
            dst_layout = vk::ImageLayout::GENERAL;
        }

        self.use_image_for_copy(src_layout, vk::AccessFlags::TRANSFER_READ, my_src, src_range);
        self.use_image_for_copy(dst_layout, vk::AccessFlags::TRANSFER_WRITE, my_dst, dst_range);

        let src_origin: [u32; 3] = src_origin.into_with_pad(0);
        let dst_origin: [u32; 3] = dst_origin.into_with_pad(0);
        let size: [u32; 3] = size.into_with_pad(1);

        assert_eq!(src_range.layers.len(), dst_range.layers.len());

        let src_aspect = my_src.aspects();
        let dst_aspect = my_dst.aspects();

        assert_eq!(
            src_aspect, dst_aspect,
            "source and destination format must match"
        );

        let vk_device = self.device.vk_device();

        unsafe {
            vk_device.cmd_copy_image(
                self.vk_cmd_buffer(),
                my_src.vk_image(),
                src_layout,
                my_dst.vk_image(),
                dst_layout,
                &[vk::ImageCopy {
                    src_subresource: my_src.resolve_vk_subresource_layers(src_range, src_aspect),
                    src_offset: vk::Offset3D {
                        x: src_origin[0] as i32,
                        y: src_origin[1] as i32,
                        z: src_origin[2] as i32,
                    },
                    dst_subresource: my_dst.resolve_vk_subresource_layers(dst_range, dst_aspect),
                    dst_offset: vk::Offset3D {
                        x: dst_origin[0] as i32,
                        y: dst_origin[1] as i32,
                        z: dst_origin[2] as i32,
                    },
                    extent: vk::Extent3D {
                        width: size[0],
                        height: size[1],
                        depth: size[2],
                    },
                }],
            );
        }
    }
}
