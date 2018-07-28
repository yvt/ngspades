//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `RenderPass` for Vulkan.
use ash::version::*;
use ash::vk;
use refeq::RefEqArc;
use std::ops;

use crate::device::DeviceRef;
use crate::formats::translate_image_format;
use crate::image::Image;
use zangfx_base as base;
use zangfx_base::{interfaces, vtable_for, zangfx_impl_handle, zangfx_impl_object};
use zangfx_base::{Error, ErrorKind, Result};
use zangfx_common::IntoWithPad;

use crate::utils::{
    translate_access_type_flags, translate_generic_error_unwrap, translate_image_layout,
    translate_pipeline_stage_flags,
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

        if from == self.subpass {
            unimplemented!();
        }

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

    fn subpass_color_targets(&mut self, targets: &[Option<base::RenderPassTargetIndex>]) {
        assert_eq!(self.subpass, 0);

        self.color_attachments.clear();
        self.color_attachments
            .extend(targets.iter().map(|maybe_target| {
                if let &Some(i) = maybe_target {
                    vk::AttachmentReference {
                        attachment: i as u32,
                        layout: unimplemented!(), // translate_image_layout(layout, false),
                    }
                } else {
                    vk::AttachmentReference {
                        attachment: vk::VK_ATTACHMENT_UNUSED,
                        layout: vk::ImageLayout::Undefined,
                    }
                }
            }));
    }

    fn subpass_ds_target(&mut self, target: Option<base::RenderPassTargetIndex>) {
        assert_eq!(self.subpass, 0);

        self.depth_stencil_attachment = target.map(|i| vk::AttachmentReference {
            attachment: i as u32,
            layout: unimplemented!(), // translate_image_layout(layout, true),
        });
    }

    fn build(&mut self) -> Result<base::RenderPassRef> {
        let vk_device = self.device.vk_device();

        let vk_subpass = vk::SubpassDescription {
            flags: vk::SubpassDescriptionFlags::empty(),
            pipeline_bind_point: vk::PipelineBindPoint::Graphics,
            input_attachment_count: 0,
            p_input_attachments: ::null(),
            color_attachment_count: self.color_attachments.len() as u32,
            p_color_attachments: self.color_attachments.as_ptr(),
            p_resolve_attachments: ::null(),
            p_depth_stencil_attachment: self
                .depth_stencil_attachment
                .as_ref()
                .map(|x| x as *const _)
                .unwrap_or(::null()),
            preserve_attachment_count: 0,
            p_preserve_attachments: ::null(),
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

        // The number of color attachments for subpass 0
        let num_color_attachments = self.color_attachments.len();

        let vk_render_pass = unsafe { vk_device.create_render_pass(&vk_info, None) }
            .map_err(translate_generic_error_unwrap)?;

        Ok(
            unsafe { RenderPass::from_raw(self.device, vk_render_pass, num_color_attachments) }
                .into(),
        )
    }
}

#[derive(Debug, Clone)]
struct RenderPassTargetBuilder {
    vk_desc: vk::AttachmentDescription,
    format: base::ImageFormat,
    initial_layout: base::ImageLayout,
    final_layout: base::ImageLayout,
}

zangfx_impl_object! { RenderPassTargetBuilder: dyn base::RenderPassTarget, dyn (crate::Debug) }

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
            initial_layout: unimplemented!(), //base::ImageLayout::Undefined,
            final_layout: unimplemented!(),   //base::ImageLayout::ShaderRead,
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

    /*
    fn set_initial_layout(&mut self, v: base::ImageLayout) -> &mut dyn base::RenderPassTarget {
        // The actual layout cannot be decided without knowing whether the image
        // has the depth/stencil format.
        self.initial_layout = v;
        self
    }
    fn set_final_layout(&mut self, v: base::ImageLayout) -> &mut dyn base::RenderPassTarget {
        // The actual layout cannot be decided without knowing whether the image
        // has the depth/stencil format.
        self.final_layout = v;
        self
    } */
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

zangfx_impl_handle! { RenderPass, base::RenderPassRef }

#[derive(Debug)]
struct RenderPassData {
    device: DeviceRef,
    vk_render_pass: vk::RenderPass,
    num_color_attachments: usize,
}

impl RenderPass {
    pub(crate) unsafe fn from_raw(
        device: DeviceRef,
        vk_render_pass: vk::RenderPass,
        num_color_attachments: usize,
    ) -> Self {
        Self {
            data: RefEqArc::new(RenderPassData {
                device,
                vk_render_pass,
                num_color_attachments,
            }),
        }
    }

    pub fn vk_render_pass(&self) -> vk::RenderPass {
        self.data.vk_render_pass
    }

    pub(crate) fn num_color_attachments(&self, _subpass: usize) -> usize {
        self.data.num_color_attachments
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

/// Image views that are destroyed automatically.
#[derive(Debug)]
struct UniqueImageViews {
    device: DeviceRef,
    image_views: Vec<vk::ImageView>,
}

impl UniqueImageViews {
    unsafe fn with_capacity(device: DeviceRef, capacity: usize) -> Self {
        Self {
            device,
            image_views: Vec::with_capacity(capacity),
        }
    }
}

impl ops::Deref for UniqueImageViews {
    type Target = Vec<vk::ImageView>;

    fn deref(&self) -> &Self::Target {
        &self.image_views
    }
}

impl ops::DerefMut for UniqueImageViews {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.image_views
    }
}

impl Drop for UniqueImageViews {
    fn drop(&mut self) {
        let vk_device = self.device.vk_device();
        for image_view in self.image_views.drain(..) {
            unsafe {
                vk_device.destroy_image_view(image_view, None);
            }
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
    pub(super) unsafe fn new(device: DeviceRef) -> Self {
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

        let mut image_views =
            unsafe { UniqueImageViews::with_capacity(self.device, self.targets.len()) };
        for target in self.targets.iter() {
            let target = target.as_ref().unwrap();

            let flags = vk::ImageViewCreateFlags::empty();
            // flags: "reserved for future use"

            let image: &Image = &target.image;

            let vk_image_view_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::ImageViewCreateInfo,
                p_next: ::null(),
                flags,
                image: image.vk_image(),
                view_type: vk::ImageViewType::Type2dArray,
                format: image.meta().format(),
                components: vk::ComponentMapping {
                    r: vk::ComponentSwizzle::Identity,
                    g: vk::ComponentSwizzle::Identity,
                    b: vk::ComponentSwizzle::Identity,
                    a: vk::ComponentSwizzle::Identity,
                },
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: image.meta().image_aspects(),
                    base_mip_level: target.mip_level,
                    base_array_layer: target.layer,
                    level_count: 1,
                    layer_count: self.num_layers,
                },
            };

            let vk_image_view = unsafe { vk_device.create_image_view(&vk_image_view_info, None) }
                .map_err(translate_generic_error_unwrap)?;
            image_views.push(vk_image_view);
        }

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
            s_type: vk::StructureType::FramebufferCreateInfo,
            p_next: ::null(),
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
                self.device,
                vk_framebuffer,
                render_pass,
                image_views,
                render_area,
                clear_values,
            )
        }.into())
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
            self.clear_value.0.depth.depth = depth;
            self.clear_value.0.depth.stencil = stencil;
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
    image_views: UniqueImageViews,
    render_area: vk::Rect2D,
    clear_values: Vec<ClearValue>,
}

impl RenderTargetTable {
    unsafe fn from_raw(
        device: DeviceRef,
        vk_framebuffer: vk::Framebuffer,
        render_pass: RenderPass,
        image_views: UniqueImageViews,
        render_area: vk::Rect2D,
        clear_values: Vec<ClearValue>,
    ) -> Self {
        Self {
            data: RefEqArc::new(RenderTargetTableData {
                device,
                vk_framebuffer,
                render_pass,
                image_views,
                render_area,
                clear_values,
            }),
        }
    }

    pub fn vk_framebuffer(&self) -> vk::Framebuffer {
        self.data.vk_framebuffer
    }

    pub(crate) fn render_pass(&self) -> &RenderPass {
        &self.data.render_pass
    }

    pub(crate) fn render_area(&self) -> &vk::Rect2D {
        &self.data.render_area
    }

    pub(crate) fn render_pass_begin_info(&self) -> vk::RenderPassBeginInfo {
        vk::RenderPassBeginInfo {
            s_type: vk::StructureType::RenderPassBeginInfo,
            p_next: ::null(),
            render_pass: self.render_pass().vk_render_pass(),
            framebuffer: self.vk_framebuffer(),
            render_area: self.render_area().clone(),
            clear_value_count: self.data.clear_values.len() as u32,
            p_clear_values: self.data.clear_values.as_ptr() as *const _,
        }
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
                    depth_stencil: self.0.depth,
                }
            })
            .finish()
    }
}
