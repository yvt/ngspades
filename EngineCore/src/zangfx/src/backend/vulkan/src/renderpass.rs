//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `RenderPass` for Vulkan.
use ash::version::*;
use ash::vk;
use refeq::RefEqArc;

use crate::device::DeviceRef;
use crate::formats::translate_image_format;
use crate::image::{Image, IMAGE_LAYOUT_COLOR_ATTACHMENT, IMAGE_LAYOUT_DS_ATTACHMENT};
use zangfx_base as base;
use zangfx_base::Result;
use zangfx_base::{zangfx_impl_handle, zangfx_impl_object};
use zangfx_common::IntoWithPad;

use crate::utils::{
    translate_access_type_flags, translate_generic_error_unwrap, translate_pipeline_stage_flags,
};

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

zangfx_impl_object! { RenderPassBuilder: dyn base::RenderPassBuilder, dyn (crate::Debug) }

impl RenderPassBuilder {
    crate fn new(device: DeviceRef) -> Self {
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
    fn target(&mut self, index: base::RenderPassTargetIndex) -> &mut dyn base::RenderPassTarget {
        if self.targets.len() <= index {
            self.targets.resize(index + 1, None);
        }
        self.targets[index] = Some(RenderPassTargetBuilder::new());
        self.targets[index].as_mut().unwrap()
    }

    fn subpass_dep(
        &mut self,
        from: base::SubpassIndex,
        src_access: base::AccessTypeFlags,
        dst_access: base::AccessTypeFlags,
    ) -> &mut dyn base::RenderPassBuilder {
        let from = from as u32;

        let src_access_mask = translate_access_type_flags(src_access);
        let dst_access_mask = translate_access_type_flags(dst_access);

        let src_stages = src_access.supported_stages();
        let dst_stages = dst_access.supported_stages();

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

    fn subpass_color_targets(&mut self, targets: &[Option<base::RenderPassTargetIndex>]) {
        assert_eq!(self.subpass, 0);

        self.color_attachments.clear();
        self.color_attachments
            .extend(targets.iter().map(|maybe_target| {
                if let &Some(i) = maybe_target {
                    vk::AttachmentReference {
                        attachment: i as u32,
                        layout: IMAGE_LAYOUT_COLOR_ATTACHMENT,
                    }
                } else {
                    vk::AttachmentReference {
                        attachment: vk::ATTACHMENT_UNUSED,
                        layout: vk::ImageLayout::UNDEFINED,
                    }
                }
            }));
    }

    fn subpass_ds_target(&mut self, target: Option<base::RenderPassTargetIndex>) {
        assert_eq!(self.subpass, 0);

        self.depth_stencil_attachment = target.map(|i| vk::AttachmentReference {
            attachment: i as u32,
            layout: IMAGE_LAYOUT_DS_ATTACHMENT,
        });
    }

    fn build(&mut self) -> Result<base::RenderPassRef> {
        let vk_device = self.device.vk_device();

        let vk_subpass = vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            input_attachment_count: 0,
            p_input_attachments: crate::null(),
            color_attachment_count: self.color_attachments.len() as u32,
            p_color_attachments: self.color_attachments.as_ptr(),
            p_resolve_attachments: crate::null(),
            p_depth_stencil_attachment: self
                .depth_stencil_attachment
                .as_ref()
                .map(|x| x as *const _)
                .unwrap_or(crate::null()),
            preserve_attachment_count: 0,
            p_preserve_attachments: crate::null(),
        };

        let vk_attachments: Vec<_> = self
            .targets
            .iter()
            .map(|target| {
                target
                    .as_ref()
                    .expect("render target bindings must be tightly arranged")
                    .vk_desc()
            })
            .collect();

        let attachment_layouts: Vec<_> = vk_attachments
            .iter()
            .map(|vk_a| [vk_a.initial_layout, vk_a.final_layout])
            .collect();

        let vk_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            p_next: crate::null(),
            flags: vk::RenderPassCreateFlags::empty(),
            attachment_count: vk_attachments.len() as u32,
            p_attachments: vk_attachments.as_ptr(),
            subpass_count: 1,
            p_subpasses: &vk_subpass,
            dependency_count: self.dependencies.len() as u32,
            p_dependencies: self.dependencies.as_ptr(),
        };

        // The number of color attachments for subpass 0
        let num_color_attachments = self.color_attachments.len();

        let vk_render_pass = unsafe { vk_device.create_render_pass(&vk_info, None) }
            .map_err(translate_generic_error_unwrap)?;

        Ok(unsafe {
            RenderPass::from_raw(
                self.device.clone(),
                vk_render_pass,
                num_color_attachments,
                attachment_layouts,
            )
        }
        .into())
    }
}

#[derive(Debug, Clone)]
struct RenderPassTargetBuilder {
    vk_desc: vk::AttachmentDescription,
    format: base::ImageFormat,
}

zangfx_impl_object! { RenderPassTargetBuilder: dyn base::RenderPassTarget, dyn (crate::Debug) }

impl RenderPassTargetBuilder {
    fn new() -> Self {
        Self {
            vk_desc: vk::AttachmentDescription {
                flags: vk::AttachmentDescriptionFlags::empty(),
                format: vk::Format::R32_SFLOAT,
                samples: vk::SampleCountFlags::TYPE_1,
                load_op: vk::AttachmentLoadOp::DONT_CARE,
                store_op: vk::AttachmentStoreOp::DONT_CARE,
                stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
                stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
                initial_layout: vk::ImageLayout::UNDEFINED,
                final_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
            // No default value is defined for `format`
            format: base::ImageFormat::RFloat32,
        }
    }

    fn vk_desc(&self) -> vk::AttachmentDescription {
        let mut vk_desc = self.vk_desc.clone();

        let format = self.format;
        let is_depth_stencil = format.has_depth() || format.has_stencil();

        let render_layout = if is_depth_stencil {
            IMAGE_LAYOUT_DS_ATTACHMENT
        } else {
            IMAGE_LAYOUT_COLOR_ATTACHMENT
        };

        vk_desc.initial_layout = if vk_desc.load_op == vk::AttachmentLoadOp::LOAD {
            render_layout
        } else {
            vk::ImageLayout::UNDEFINED
        };
        vk_desc.final_layout = render_layout;

        vk_desc
    }
}

impl base::RenderPassTarget for RenderPassTargetBuilder {
    fn set_format(&mut self, v: base::ImageFormat) -> &mut dyn base::RenderPassTarget {
        self.vk_desc.format = translate_image_format(v).expect("unsupported format");
        self
    }

    fn set_load_op(&mut self, v: base::LoadOp) -> &mut dyn base::RenderPassTarget {
        self.vk_desc.load_op = translate_load_op(v);
        self
    }
    fn set_store_op(&mut self, v: base::StoreOp) -> &mut dyn base::RenderPassTarget {
        self.vk_desc.store_op = translate_store_op(v);
        self
    }

    fn set_stencil_load_op(&mut self, v: base::LoadOp) -> &mut dyn base::RenderPassTarget {
        self.vk_desc.stencil_load_op = translate_load_op(v);
        self
    }

    fn set_stencil_store_op(&mut self, v: base::StoreOp) -> &mut dyn base::RenderPassTarget {
        self.vk_desc.stencil_store_op = translate_store_op(v);
        self
    }
}

fn translate_load_op(load_op: base::LoadOp) -> vk::AttachmentLoadOp {
    match load_op {
        base::LoadOp::Load => vk::AttachmentLoadOp::LOAD,
        base::LoadOp::DontCare => vk::AttachmentLoadOp::DONT_CARE,
        base::LoadOp::Clear => vk::AttachmentLoadOp::CLEAR,
    }
}

fn translate_store_op(store_op: base::StoreOp) -> vk::AttachmentStoreOp {
    match store_op {
        base::StoreOp::Store => vk::AttachmentStoreOp::STORE,
        base::StoreOp::DontCare => vk::AttachmentStoreOp::DONT_CARE,
    }
}

/// Implementation of `RenderPass` for Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderPass {
    data: RefEqArc<RenderPassData>,
}

zangfx_impl_handle! { RenderPass, base::RenderPassRef }

#[derive(Debug)]
struct RenderPassData {
    device: DeviceRef,
    vk_render_pass: vk::RenderPass,
    num_color_attachments: usize,
    attachment_layouts: Vec<[vk::ImageLayout; 2]>,
}

impl RenderPass {
    pub(crate) unsafe fn from_raw(
        device: DeviceRef,
        vk_render_pass: vk::RenderPass,
        num_color_attachments: usize,
        attachment_layouts: Vec<[vk::ImageLayout; 2]>,
    ) -> Self {
        Self {
            data: RefEqArc::new(RenderPassData {
                device,
                vk_render_pass,
                num_color_attachments,
                attachment_layouts,
            }),
        }
    }

    pub fn vk_render_pass(&self) -> vk::RenderPass {
        self.data.vk_render_pass
    }

    crate fn num_color_attachments(&self, _subpass: usize) -> usize {
        self.data.num_color_attachments
    }

    crate fn attachment_layouts(&self) -> &[[vk::ImageLayout; 2]] {
        &self.data.attachment_layouts
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

/// Implementation of `RenderTargetTableBuilder` for Vulkan.
#[derive(Debug)]
pub struct RenderTargetTableBuilder {
    device: DeviceRef,

    render_pass: Option<RenderPass>,
    extents: Option<[u32; 2]>,
    num_layers: u32,
    targets: Vec<Option<Target>>,
}

zangfx_impl_object! { RenderTargetTableBuilder: dyn base::RenderTargetTableBuilder, dyn (crate::Debug) }

/// Implementation of `RenderTarget` for Vulkan.
#[derive(Debug, Clone)]
struct Target {
    image: Image,
    mip_level: u32,
    layer: u32,
    clear_value: ClearValue,
}

zangfx_impl_object! { Target: dyn base::RenderTarget, dyn (crate::Debug) }

impl RenderTargetTableBuilder {
    crate fn new(device: DeviceRef) -> Self {
        Self {
            device,

            render_pass: None,
            extents: None,
            num_layers: 1,
            targets: Vec::new(),
        }
    }
}

impl base::RenderTargetTableBuilder for RenderTargetTableBuilder {
    fn render_pass(&mut self, v: &base::RenderPassRef) -> &mut dyn base::RenderTargetTableBuilder {
        let our_rp: &RenderPass = v.downcast_ref().expect("bad render pass type");
        self.render_pass = Some(our_rp.clone());
        self
    }

    fn extents(&mut self, v: &[u32]) -> &mut dyn base::RenderTargetTableBuilder {
        self.extents = Some(v.into_with_pad(1));
        self
    }

    fn num_layers(&mut self, v: u32) -> &mut dyn base::RenderTargetTableBuilder {
        self.num_layers = v;
        self
    }

    fn target(
        &mut self,
        index: base::RenderPassTargetIndex,
        view: &base::ImageRef,
    ) -> &mut dyn base::RenderTarget {
        use std::mem::uninitialized;
        if self.targets.len() <= index {
            self.targets.resize(index + 1, None);
        }

        let our_image: &Image = view.downcast_ref().expect("bad image type");
        self.targets[index] = Some(Target {
            image: our_image.clone(),
            mip_level: 0,
            layer: 0,
            clear_value: unsafe { uninitialized() },
        });

        self.targets[index].as_mut().unwrap()
    }

    fn build(&mut self) -> Result<base::RenderTargetTableRef> {
        let render_pass: RenderPass = self.render_pass.clone().expect("render_pass");
        let extents = self.extents.expect("extents");

        let vk_device = self.device.vk_device();

        let images: Vec<_> = self
            .targets
            .iter()
            .map(|target| {
                let target = target.as_ref().expect("target");

                let image = (&target.image as &dyn base::Image)
                    .build_image_view()
                    .subrange(&base::ImageSubRange {
                        layers: Some(target.layer..target.layer + self.num_layers),
                        mip_levels: Some(target.mip_level..target.mip_level + 1),
                    })
                    .image_type(base::ImageType::TwoDArray)
                    .build()?;

                let our_image: &Image = image.downcast_ref().unwrap();
                Ok(our_image.clone())
            })
            .collect::<Result<_>>()?;

        let image_views: Vec<_> = images.iter().map(|image| image.vk_image_view()).collect();

        let render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D {
                width: extents[0],
                height: extents[1],
            },
        };

        let clear_values = self
            .targets
            .iter()
            .map(|target| target.as_ref().unwrap().clear_value.clone())
            .collect();

        let vk_info = vk::FramebufferCreateInfo {
            s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
            p_next: crate::null(),
            flags: vk::FramebufferCreateFlags::empty(),
            render_pass: render_pass.vk_render_pass(),
            attachment_count: self.targets.len() as u32,
            p_attachments: image_views.as_ptr(),
            width: extents[0],
            height: extents[1],
            layers: self.num_layers,
        };

        let vk_framebuffer = unsafe { vk_device.create_framebuffer(&vk_info, None) }
            .map_err(translate_generic_error_unwrap)?;

        Ok(unsafe {
            RenderTargetTable::from_raw(
                self.device.clone(),
                vk_framebuffer,
                render_pass,
                images,
                render_area,
                clear_values,
            )
        }
        .into())
    }
}

impl base::RenderTarget for Target {
    fn mip_level(&mut self, v: u32) -> &mut dyn base::RenderTarget {
        self.mip_level = v;
        self
    }

    fn layer(&mut self, v: u32) -> &mut dyn base::RenderTarget {
        self.layer = v;
        self
    }

    fn clear_float(&mut self, v: &[f32]) -> &mut dyn base::RenderTarget {
        unsafe {
            self.clear_value.0.color.float32.copy_from_slice(&v[0..4]);
        }
        self
    }

    fn clear_uint(&mut self, v: &[u32]) -> &mut dyn base::RenderTarget {
        unsafe {
            self.clear_value.0.color.uint32.copy_from_slice(&v[0..4]);
        }
        self
    }

    fn clear_sint(&mut self, v: &[i32]) -> &mut dyn base::RenderTarget {
        unsafe {
            self.clear_value.0.color.int32.copy_from_slice(&v[0..4]);
        }
        self
    }

    fn clear_depth_stencil(&mut self, depth: f32, stencil: u32) -> &mut dyn base::RenderTarget {
        unsafe {
            self.clear_value.0.depth_stencil.depth = depth;
            self.clear_value.0.depth_stencil.stencil = stencil;
        }
        self
    }
}

/// Implementation of `RenderTargetTable` for Vulkan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderTargetTable {
    data: RefEqArc<RenderTargetTableData>,
}

zangfx_impl_handle! { RenderTargetTable, base::RenderTargetTableRef }

#[derive(Debug)]
struct RenderTargetTableData {
    device: DeviceRef,
    vk_framebuffer: vk::Framebuffer,
    render_pass: RenderPass,
    /// Contains the attachments of the framebuffer.
    images: Vec<Image>,
    render_area: vk::Rect2D,
    clear_values: Vec<ClearValue>,
}

impl RenderTargetTable {
    unsafe fn from_raw(
        device: DeviceRef,
        vk_framebuffer: vk::Framebuffer,
        render_pass: RenderPass,
        images: Vec<Image>,
        render_area: vk::Rect2D,
        clear_values: Vec<ClearValue>,
    ) -> Self {
        Self {
            data: RefEqArc::new(RenderTargetTableData {
                device,
                vk_framebuffer,
                render_pass,
                images,
                render_area,
                clear_values,
            }),
        }
    }

    pub fn vk_framebuffer(&self) -> vk::Framebuffer {
        self.data.vk_framebuffer
    }

    crate fn render_pass(&self) -> &RenderPass {
        &self.data.render_pass
    }

    crate fn render_area(&self) -> &vk::Rect2D {
        &self.data.render_area
    }

    crate fn render_pass_begin_info(&self) -> vk::RenderPassBeginInfo {
        vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
            p_next: crate::null(),
            render_pass: self.render_pass().vk_render_pass(),
            framebuffer: self.vk_framebuffer(),
            render_area: self.render_area().clone(),
            clear_value_count: self.data.clear_values.len() as u32,
            p_clear_values: self.data.clear_values.as_ptr() as *const _,
        }
    }

    crate fn images(&self) -> &[Image] {
        &self.data.images
    }
}

impl Drop for RenderTargetTableData {
    fn drop(&mut self) {
        let vk_device = self.device.vk_device();
        unsafe {
            vk_device.destroy_framebuffer(self.vk_framebuffer, None);
        }
    }
}

/// `Debug` wrapper for `vk::ClearValue`
#[derive(Clone)]
#[repr(C)]
struct ClearValue(vk::ClearValue);

use std::fmt;
impl fmt::Debug for ClearValue {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Debug)]
        struct Values {
            float32: [f32; 4],
            uint32: [u32; 4],
            int32: [i32; 4],
            depth_stencil: vk::ClearDepthStencilValue,
        }
        fmt.debug_tuple("Target")
            .field(unsafe {
                &Values {
                    float32: self.0.color.float32,
                    uint32: self.0.color.uint32,
                    int32: self.0.color.int32,
                    depth_stencil: self.0.depth_stencil,
                }
            })
            .finish()
    }
}
