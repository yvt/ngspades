//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use ash::version::DeviceV1_0;
use ash::vk;
use std::ptr;

use {RefEqArc, DeviceRef, AshDevice, translate_generic_error_unwrap};
use imp;
use imp::{translate_image_format, translate_image_layout, translate_access_type_flags,
          translate_pipeline_stage_flags};

pub struct RenderPass<T: DeviceRef> {
    data: RefEqArc<RenderPassData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for RenderPass<T> => data
}

#[derive(Debug)]
struct RenderPassData<T: DeviceRef> {
    device_ref: T,
    handle: vk::RenderPass,
    num_subpasses: usize,
}

impl<T: DeviceRef> Drop for RenderPassData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_render_pass(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> core::RenderPass for RenderPass<T> {}

impl<T: DeviceRef> core::Marker for RenderPass<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> RenderPass<T> {
    pub(crate) fn new(device_ref: &T, desc: &core::RenderPassDescription) -> core::Result<Self> {
        let attachments: Vec<_> = desc.attachments
            .iter()
            .map(|ad| {
                vk::AttachmentDescription {
                    flags: if ad.may_alias {
                        vk::ATTACHMENT_DESCRIPTION_MAY_ALIAS_BIT
                    } else {
                        vk::AttachmentDescriptionFlags::empty()
                    },
                    format: translate_image_format(ad.format).expect("unsupported format"),
                    samples: vk::SAMPLE_COUNT_1_BIT,
                    load_op: translate_load_op(ad.load_op),
                    store_op: translate_store_op(ad.store_op),
                    stencil_load_op: translate_load_op(ad.stencil_load_op),
                    stencil_store_op: translate_store_op(ad.stencil_store_op),
                    initial_layout: translate_image_layout(ad.initial_layout),
                    final_layout: translate_image_layout(ad.final_layout),
                }
            })
            .collect();

        let mut att_refs = Vec::new();
        let mut att_idxs = Vec::new();

        let subpass_refs: Vec<_> = desc.subpasses
            .iter()
            .map(|spd| {
                let i = att_refs.len();
                att_refs.extend(spd.input_attachments.iter().map(
                    translate_attachment_reference,
                ));
                let ia_i = i..att_refs.len();

                let i = att_refs.len();
                att_refs.extend(spd.color_attachments.iter().map(
                    translate_attachment_reference,
                ));
                let ca_i = i..att_refs.len();

                let i = att_refs.len();
                let dsa_i = if let Some(ref ar) = spd.depth_stencil_attachment {
                    att_refs.push(translate_attachment_reference(ar));
                    Some(i)
                } else {
                    None
                };

                let i = att_idxs.len();
                att_idxs.extend(spd.preserve_attachments.iter().map(|&x| x as u32));
                let pa_i = i..att_idxs.len();

                (ia_i, ca_i, dsa_i, pa_i)
            })
            .collect();

        let subpasses: Vec<_> = subpass_refs
            .into_iter()
            .map(|(ia_i, ca_i, dsa_i, pa_i)| {
                vk::SubpassDescription {
                    flags: vk::SubpassDescriptionFlags::empty(),
                    pipeline_bind_point: vk::PipelineBindPoint::Graphics,
                    input_attachment_count: ia_i.len() as u32,
                    p_input_attachments: att_refs.as_ptr().wrapping_offset(ia_i.start as isize),
                    color_attachment_count: ca_i.len() as u32,
                    p_color_attachments: att_refs.as_ptr().wrapping_offset(ca_i.start as isize),
                    p_resolve_attachments: ptr::null(),
                    p_depth_stencil_attachment: if let Some(dsa_i) = dsa_i {
                        &att_refs[dsa_i]
                    } else {
                        ptr::null()
                    },
                    preserve_attachment_count: pa_i.len() as u32,
                    p_preserve_attachments: att_idxs.as_ptr().wrapping_offset(pa_i.start as isize),
                }
            })
            .collect();

        let deps: Vec<_> = desc.dependencies
            .iter()
            .map(|dep| {
                use core::RenderSubpassDependencyTarget::*;
                vk::SubpassDependency {
                    src_subpass: match dep.source {
                        Subpass { index } => index as u32,
                        External => vk::VK_SUBPASS_EXTERNAL,
                    },
                    dst_subpass: match dep.destination {
                        Subpass { index } => index as u32,
                        External => vk::VK_SUBPASS_EXTERNAL,
                    },
                    src_stage_mask: translate_pipeline_stage_flags(dep.source_stage_mask),
                    dst_stage_mask: translate_pipeline_stage_flags(dep.destination_stage_mask),
                    src_access_mask: translate_access_type_flags(dep.source_access_mask),
                    dst_access_mask: translate_access_type_flags(dep.destination_access_mask),
                    dependency_flags: vk::DependencyFlags::empty(),
                }
            })
            .collect();

        let info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RenderPassCreateInfo,
            p_next: ptr::null(),
            flags: vk::RenderPassCreateFlags::empty(),
            attachment_count: attachments.len() as u32,
            p_attachments: attachments.as_ptr(),
            subpass_count: subpasses.len() as u32,
            p_subpasses: subpasses.as_ptr(),
            dependency_count: deps.len() as u32,
            p_dependencies: deps.as_ptr(),
        };

        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe { device.create_render_pass(&info, device_ref.allocation_callbacks()) }
                .map_err(translate_generic_error_unwrap)?;
        }

        Ok(RenderPass {
            data: RefEqArc::new(RenderPassData {
                device_ref: device_ref.clone(),
                handle,
                num_subpasses: subpasses.len(),
            }),
        })
    }

    pub fn handle(&self) -> vk::RenderPass {
        self.data.handle
    }
}

fn translate_attachment_reference(
    value: &core::RenderPassAttachmentReference,
) -> vk::AttachmentReference {
    vk::AttachmentReference {
        attachment: value.attachment_index.map(|x| x as u32).unwrap_or(
            vk::VK_ATTACHMENT_UNUSED,
        ),
        layout: translate_image_layout(value.layout),
    }
}

fn translate_load_op(value: core::AttachmentLoadOp) -> vk::AttachmentLoadOp {
    match value {
        core::AttachmentLoadOp::Load => vk::AttachmentLoadOp::Load,
        core::AttachmentLoadOp::Clear => vk::AttachmentLoadOp::Clear,
        core::AttachmentLoadOp::DontCare => vk::AttachmentLoadOp::DontCare,
    }
}

fn translate_store_op(value: core::AttachmentStoreOp) -> vk::AttachmentStoreOp {
    match value {
        core::AttachmentStoreOp::Store => vk::AttachmentStoreOp::Store,
        core::AttachmentStoreOp::DontCare => vk::AttachmentStoreOp::DontCare,
    }
}

pub struct Framebuffer<T: DeviceRef> {
    data: RefEqArc<FramebufferData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Framebuffer<T> => data
}

#[derive(Debug)]
struct FramebufferData<T: DeviceRef> {
    device_ref: T,
    handle: vk::Framebuffer,
    clear_values: Vec<vk::ClearValue>,
    render_pass: RenderPass<T>,
    extent: vk::Extent2D,
    ivs: Vec<imp::ImageView<T>>,
}

impl<T: DeviceRef> Drop for FramebufferData<T> {
    fn drop(&mut self) {
        let device: &AshDevice = self.device_ref.device();
        unsafe { device.destroy_framebuffer(self.handle, self.device_ref.allocation_callbacks()) };
    }
}

impl<T: DeviceRef> core::Framebuffer for Framebuffer<T> {}

impl<T: DeviceRef> core::Marker for Framebuffer<T> {
    fn set_label(&self, _: Option<&str>) {
        // TODO: set_label
    }
}

impl<T: DeviceRef> Framebuffer<T> {
    pub(crate) fn new(desc: &imp::FramebufferDescription<T>) -> core::Result<Self> {
        let device_ref: T = desc.render_pass.data.device_ref.clone();

        let ivs: Vec<_> = desc.attachments
            .iter()
            .map(|ad| ad.image_view.clone())
            .collect();
        let vk_ivs: Vec<_> = ivs.iter().map(imp::ImageView::handle).collect();

        use core::ClearValues::*;
        let clear_values: Vec<_> = desc.attachments
            .iter()
            .map(|ad| match ad.clear_values {
                ColorFloat(ref values) => vk::ClearValue::new_color(
                    vk::ClearColorValue::new_float32(values.clone()),
                ),
                ColorUnsignedInteger(ref values) => vk::ClearValue::new_color(
                    vk::ClearColorValue::new_uint32(values.clone()),
                ),
                ColorSignedInteger(ref values) => vk::ClearValue::new_color(
                    vk::ClearColorValue::new_int32(values.clone()),
                ),
                DepthStencil(depth, stencil) => vk::ClearValue::new_depth_stencil(
                    vk::ClearDepthStencilValue { depth, stencil },
                ),
            })
            .collect();

        let info = vk::FramebufferCreateInfo {
            s_type: vk::StructureType::FramebufferCreateInfo,
            p_next: ptr::null(),
            flags: vk::FramebufferCreateFlags::empty(),
            render_pass: desc.render_pass.handle(),
            attachment_count: vk_ivs.len() as u32,
            p_attachments: vk_ivs.as_ptr(),
            width: desc.width,
            height: desc.height,
            layers: desc.num_layers,
        };

        let handle;
        {
            let device: &AshDevice = device_ref.device();
            handle = unsafe { device.create_framebuffer(&info, device_ref.allocation_callbacks()) }
                .map_err(translate_generic_error_unwrap)?;
        }

        Ok(Framebuffer {
            data: RefEqArc::new(FramebufferData {
                device_ref,
                handle,
                clear_values,
                render_pass: desc.render_pass.clone(),
                extent: vk::Extent2D {
                    width: desc.width,
                    height: desc.height,
                },
                ivs,
            }),
        })
    }
    pub(crate) fn num_subpasses(&self) -> usize {
        self.data.render_pass.data.num_subpasses
    }

    pub(crate) fn render_pass_begin_info(&self) -> vk::RenderPassBeginInfo {
        vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RenderPassBeginInfo,
            p_next: ptr::null(),
            render_pass: self.data.render_pass.handle(),
            framebuffer: self.data.handle,
            render_area: vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.data.extent.clone(),
            },
            clear_value_count: self.data.clear_values.len() as u32,
            p_clear_values: self.data.clear_values.as_ptr(),
        }
    }

    pub fn handle(&self) -> vk::Framebuffer {
        self.data.handle
    }

    pub(crate) fn render_pass_handle(&self) -> vk::RenderPass {
        self.data.render_pass.handle()
    }
}
