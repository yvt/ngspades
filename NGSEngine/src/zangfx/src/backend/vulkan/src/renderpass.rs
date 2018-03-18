//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `RenderPass` for Vulkan.
use ash::vk;
use ash::version::*;
use refeq::RefEqArc;

use base;
use common::Result;
use device::DeviceRef;
use formats::translate_image_format;

use utils::{translate_access_type_flags, translate_generic_error_unwrap, translate_image_layout,
            translate_pipeline_stage_flags};

/// Implementation of `RenderPassBuilder` for Vulkan.
#[derive(Debug)]
pub struct RenderPassBuilder {
    device: DeviceRef,
    targets: Vec<Option<RenderPassTargetBuilder>>,

    /// The current subpass index or `vk::VK_SUBPASS_EXTERNAL`.
    subpass: u32,
    dependencies: Vec<vk::SubpassDependency>,

    /// The color attachment for subpass 0.
    color_attachments: Vec<vk::AttachmentReference>,
    /// The depth/stencil attachment for subpass 0.
    depth_stencil_attachment: Option<vk::AttachmentReference>,
}

zangfx_impl_object! { RenderPassBuilder: base::RenderPassBuilder, ::Debug }

impl RenderPassBuilder {
    pub(super) unsafe fn new(device: DeviceRef) -> Self {
        Self {
            device,
            targets: Vec::new(),
            subpass: 0,
            dependencies: Vec::new(),
            color_attachments: Vec::new(),
            depth_stencil_attachment: None,
        }
    }
}

impl base::RenderPassBuilder for RenderPassBuilder {
    fn target(&mut self, index: base::RenderPassTargetIndex) -> &mut base::RenderPassTarget {
        if self.targets.len() <= index {
            self.targets.resize(index + 1, None);
        }
        self.targets[index] = Some(RenderPassTargetBuilder::new());
        self.targets[index].as_mut().unwrap()
    }

    fn end(&mut self) -> &mut base::RenderPassBuilder {
        self.subpass = vk::VK_SUBPASS_EXTERNAL;
        self
    }

    fn subpass_dep(
        &mut self,
        from: Option<base::SubpassIndex>,
        src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
    ) -> &mut base::RenderPassBuilder {
        let from = if let Some(from) = from {
            from as u32
        } else {
            vk::VK_SUBPASS_EXTERNAL
        };

        assert_ne!(from, self.subpass);

        let src_access_mask = translate_access_type_flags(src_access);
        let dst_access_mask = translate_access_type_flags(dst_access);

        let src_stages = base::AccessType::union_supported_stages(src_access);
        let dst_stages = base::AccessType::union_supported_stages(dst_access);

        let src_stage_mask = translate_pipeline_stage_flags(src_stages);
        let dst_stage_mask = translate_pipeline_stage_flags(dst_stages);

        self.dependencies.push(vk::SubpassDependency {
            src_subpass: from,
            dst_subpass: self.subpass,
            src_stage_mask,
            dst_stage_mask,
            src_access_mask,
            dst_access_mask,
            dependency_flags: vk::DependencyFlags::empty(),
        });

        self
    }

    fn subpass_color_targets(
        &mut self,
        targets: &[Option<(base::RenderPassTargetIndex, base::ImageLayout)>],
    ) -> &mut base::RenderPassBuilder {
        assert_eq!(self.subpass, 0);

        self.color_attachments.clear();
        self.color_attachments
            .extend(targets.iter().map(|maybe_target| {
                if let &Some((i, layout)) = maybe_target {
                    vk::AttachmentReference {
                        attachment: i as u32,
                        layout: translate_image_layout(layout, false),
                    }
                } else {
                    vk::AttachmentReference {
                        attachment: vk::VK_ATTACHMENT_UNUSED,
                        layout: vk::ImageLayout::Undefined,
                    }
                }
            }));

        self
    }

    fn subpass_ds_target(
        &mut self,
        target: Option<(base::RenderPassTargetIndex, base::ImageLayout)>,
    ) -> &mut base::RenderPassBuilder {
        assert_eq!(self.subpass, 0);

        self.depth_stencil_attachment = target.map(|(i, layout)| vk::AttachmentReference {
            attachment: i as u32,
            layout: translate_image_layout(layout, true),
        });

        self
    }

    fn build(&mut self) -> Result<base::RenderPass> {
        let vk_device = self.device.vk_device();

        let vk_subpass = vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::Graphics,
            input_attachment_count: 0,
            p_input_attachments: ::null(),
            color_attachment_count: self.color_attachments.len() as u32,
            p_color_attachments: self.color_attachments.as_ptr(),
            p_resolve_attachments: ::null(),
            p_depth_stencil_attachment: self.depth_stencil_attachment
                .as_ref()
                .map(|x| x as *const _)
                .unwrap_or(::null()),
            preserve_attachment_count: 0,
            p_preserve_attachments: ::null(),
        };

        let vk_attachments: Vec<_> = self.targets
            .iter()
            .map(|target| {
                target
                    .as_ref()
                    .expect("render target bindings must be tightly arranged")
                    .vk_desc()
            })
            .collect();

        let vk_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RenderPassCreateInfo,
            p_next: ::null(),
            flags: vk::RenderPassCreateFlags::empty(),
            attachment_count: vk_attachments.len() as u32,
            p_attachments: vk_attachments.as_ptr(),
            subpass_count: 1,
            p_subpasses: &vk_subpass,
            dependency_count: self.dependencies.len() as u32,
            p_dependencies: self.dependencies.as_ptr(),
        };

        let vk_render_pass = unsafe { vk_device.create_render_pass(&vk_info, None) }
            .map_err(translate_generic_error_unwrap)?;

        Ok(unsafe { RenderPass::from_raw(self.device, vk_render_pass) }.into())
    }
}

#[derive(Debug, Clone)]
struct RenderPassTargetBuilder {
    vk_desc: vk::AttachmentDescription,
    format: base::ImageFormat,
    initial_layout: base::ImageLayout,
    final_layout: base::ImageLayout,
}

zangfx_impl_object! { RenderPassTargetBuilder: base::RenderPassTarget, ::Debug }

impl RenderPassTargetBuilder {
    fn new() -> Self {
        Self {
            vk_desc: vk::AttachmentDescription {
                flags: vk::AttachmentDescriptionFlags::empty(),
                format: vk::Format::R32Sfloat,
                samples: vk::SAMPLE_COUNT_1_BIT,
                load_op: vk::AttachmentLoadOp::DontCare,
                store_op: vk::AttachmentStoreOp::DontCare,
                stencil_load_op: vk::AttachmentLoadOp::DontCare,
                stencil_store_op: vk::AttachmentStoreOp::DontCare,
                initial_layout: vk::ImageLayout::Undefined,
                final_layout: vk::ImageLayout::ShaderReadOnlyOptimal,
            },
            // No default value is defined for `format`
            format: base::ImageFormat::RFloat32,
            initial_layout: base::ImageLayout::Undefined,
            final_layout: base::ImageLayout::ShaderRead,
        }
    }

    fn vk_desc(&self) -> vk::AttachmentDescription {
        let mut vk_desc = self.vk_desc.clone();

        let format = self.format;
        let is_depth_stencil = format.has_depth() || format.has_stencil();
        vk_desc.initial_layout = translate_image_layout(self.initial_layout, is_depth_stencil);
        vk_desc.final_layout = translate_image_layout(self.final_layout, is_depth_stencil);

        vk_desc
    }
}

impl base::RenderPassTarget for RenderPassTargetBuilder {
    fn set_format(&mut self, v: base::ImageFormat) -> &mut base::RenderPassTarget {
        self.vk_desc.format = translate_image_format(v).expect("unsupported format");
        self
    }

    fn set_load_op(&mut self, v: base::LoadOp) -> &mut base::RenderPassTarget {
        self.vk_desc.load_op = translate_load_op(v);
        self
    }
    fn set_store_op(&mut self, v: base::StoreOp) -> &mut base::RenderPassTarget {
        self.vk_desc.store_op = translate_store_op(v);
        self
    }

    fn set_stencil_load_op(&mut self, v: base::LoadOp) -> &mut base::RenderPassTarget {
        self.vk_desc.stencil_load_op = translate_load_op(v);
        self
    }

    fn set_stencil_store_op(&mut self, v: base::StoreOp) -> &mut base::RenderPassTarget {
        self.vk_desc.stencil_store_op = translate_store_op(v);
        self
    }

    fn set_initial_layout(&mut self, v: base::ImageLayout) -> &mut base::RenderPassTarget {
        // The actual layout cannot be decided without knowing whether the image
        // has the depth/stencil format.
        self.initial_layout = v;
        self
    }
    fn set_final_layout(&mut self, v: base::ImageLayout) -> &mut base::RenderPassTarget {
        // The actual layout cannot be decided without knowing whether the image
        // has the depth/stencil format.
        self.final_layout = v;
        self
    }
}

fn translate_load_op(load_op: base::LoadOp) -> vk::AttachmentLoadOp {
    match load_op {
        base::LoadOp::Load => vk::AttachmentLoadOp::Load,
        base::LoadOp::DontCare => vk::AttachmentLoadOp::DontCare,
        base::LoadOp::Clear => vk::AttachmentLoadOp::Clear,
    }
}

fn translate_store_op(store_op: base::StoreOp) -> vk::AttachmentStoreOp {
    match store_op {
        base::StoreOp::Store => vk::AttachmentStoreOp::Store,
        base::StoreOp::DontCare => vk::AttachmentStoreOp::DontCare,
    }
}

/// Implementation of `RenderPass` for Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderPass {
    data: RefEqArc<RenderPassData>,
}

zangfx_impl_handle! { RenderPass, base::RenderPass }

#[derive(Debug)]
struct RenderPassData {
    device: DeviceRef,
    vk_render_pass: vk::RenderPass,
}

impl RenderPass {
    pub(crate) unsafe fn from_raw(device: DeviceRef, vk_render_pass: vk::RenderPass) -> Self {
        Self {
            data: RefEqArc::new(RenderPassData {
                device,
                vk_render_pass,
            }),
        }
    }

    pub fn vk_render_pass(&self) -> vk::RenderPass {
        self.data.vk_render_pass
    }
}

impl Drop for RenderPassData {
    fn drop(&mut self) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.destroy_render_pass(self.vk_render_pass, None);
        }
    }
}
