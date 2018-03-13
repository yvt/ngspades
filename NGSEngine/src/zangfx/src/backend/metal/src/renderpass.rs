//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::Arc;
use metal;

use base;
use common::{Error, ErrorKind, Result};

use formats::translate_image_format;
use image::Image;
use utils::{nil_error, OCPtr};

/// Implementation of `RenderPassBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct RenderPassBuilder {
    targets: Vec<Option<RenderPassTargetBuilder>>,
    subpass_color_targets: Vec<Option<usize>>,
    subpass_ds_target: Option<usize>,
}

zangfx_impl_object! { RenderPassBuilder: base::RenderPassBuilder, ::Debug }

impl RenderPassBuilder {
    /// Construct a `RenderPassBuilder`.
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            subpass_color_targets: Vec::new(),
            subpass_ds_target: None,
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
        self
    }

    fn subpass_dep(
        &mut self,
        _from: Option<base::SubpassIndex>,
        _src_access: base::AccessTypeFlags,
        _dst_access: base::AccessTypeFlags,
    ) -> &mut base::RenderPassBuilder {
        // No-op: all barriers are expressed by other means on Metal
        self
    }

    fn subpass_color_targets(
        &mut self,
        targets: &[Option<(base::RenderPassTargetIndex, base::ImageLayout)>],
    ) -> &mut base::RenderPassBuilder {
        self.subpass_color_targets = targets.iter().map(|x| x.map(|(i, _)| i)).collect();
        self
    }

    fn subpass_ds_target(
        &mut self,
        target: Option<(base::RenderPassTargetIndex, base::ImageLayout)>,
    ) -> &mut base::RenderPassBuilder {
        self.subpass_ds_target = target.map(|(i, _)| i);
        self
    }

    fn build(&mut self) -> Result<base::RenderPass> {
        let ref targets = self.targets;

        for target in targets.iter() {
            if let &Some(ref target) = target {
                if target.format.is_none() {
                    return Err(Error::with_detail(ErrorKind::InvalidUsage, "format"));
                }
            }
        }

        let colors = self.subpass_color_targets
            .iter()
            .map(|i_or_none| {
                i_or_none.map(|i| {
                    let target = targets[i].as_ref().unwrap();
                    PassTarget {
                        index: i,
                        format: translate_image_format(target.format.unwrap())
                            .expect("unsupported image format"),
                        load: translate_load_op(target.load_op),
                        store: translate_store_op(target.store_op),
                    }
                })
            })
            .collect();

        let depth = self.subpass_ds_target.map(|i| {
            let target = targets[i].as_ref().unwrap();
            PassTarget {
                index: i,
                format: translate_image_format(target.format.unwrap())
                    .expect("unsupported image format"),
                load: translate_load_op(target.load_op),
                store: translate_store_op(target.store_op),
            }
        });
        let stencil = self.subpass_ds_target.map(|i| {
            let target = targets[i].as_ref().unwrap();
            PassTarget {
                index: i,
                format: translate_image_format(target.format.unwrap())
                    .expect("unsupported image format"),
                load: translate_load_op(target.stencil_load_op),
                store: translate_store_op(target.stencil_store_op),
            }
        });

        let data = RenderPassData {
            colors,
            depth,
            stencil,
        };

        Ok(RenderPass {
            data: Arc::new(data),
        }.into())
    }
}

fn translate_load_op(load_op: base::LoadOp) -> metal::MTLLoadAction {
    match load_op {
        base::LoadOp::Load => metal::MTLLoadAction::Load,
        base::LoadOp::DontCare => metal::MTLLoadAction::DontCare,
        base::LoadOp::Clear => metal::MTLLoadAction::Clear,
    }
}

fn translate_store_op(store_op: base::StoreOp) -> metal::MTLStoreAction {
    match store_op {
        base::StoreOp::Store => metal::MTLStoreAction::Store,
        base::StoreOp::DontCare => metal::MTLStoreAction::DontCare,
    }
}

/// Implementation of `RenderPassTarget` for Metal.
#[derive(Debug, Clone)]
struct RenderPassTargetBuilder {
    format: Option<base::ImageFormat>,
    load_op: base::LoadOp,
    store_op: base::StoreOp,
    stencil_load_op: base::LoadOp,
    stencil_store_op: base::StoreOp,
}

zangfx_impl_object! { RenderPassTargetBuilder: base::RenderPassTarget, ::Debug }

unsafe impl Send for RenderPassTargetBuilder {}
unsafe impl Sync for RenderPassTargetBuilder {}

impl RenderPassTargetBuilder {
    fn new() -> Self {
        Self {
            format: None,
            load_op: base::LoadOp::DontCare,
            store_op: base::StoreOp::DontCare,
            stencil_load_op: base::LoadOp::DontCare,
            stencil_store_op: base::StoreOp::DontCare,
        }
    }
}

impl base::RenderPassTarget for RenderPassTargetBuilder {
    fn set_format(&mut self, v: base::ImageFormat) -> &mut base::RenderPassTarget {
        self.format = Some(v);
        self
    }

    fn set_load_op(&mut self, v: base::LoadOp) -> &mut base::RenderPassTarget {
        self.load_op = v;
        self
    }
    fn set_store_op(&mut self, v: base::StoreOp) -> &mut base::RenderPassTarget {
        self.store_op = v;
        self
    }

    fn set_stencil_load_op(&mut self, v: base::LoadOp) -> &mut base::RenderPassTarget {
        self.stencil_load_op = v;
        self
    }

    fn set_stencil_store_op(&mut self, v: base::StoreOp) -> &mut base::RenderPassTarget {
        self.stencil_store_op = v;
        self
    }

    fn set_initial_layout(&mut self, _: base::ImageLayout) -> &mut base::RenderPassTarget {
        self
    }
    fn set_final_layout(&mut self, _: base::ImageLayout) -> &mut base::RenderPassTarget {
        self
    }
}

/// Implementation of `RenderPass` for Metal.
#[derive(Debug, Clone)]
pub struct RenderPass {
    data: Arc<RenderPassData>,
}

zangfx_impl_handle! { RenderPass, base::RenderPass }

#[derive(Debug)]
struct RenderPassData {
    colors: Vec<Option<PassTarget>>,
    depth: Option<PassTarget>,
    stencil: Option<PassTarget>,
}

#[derive(Debug, Clone)]
struct PassTarget {
    index: base::RenderPassTargetIndex,
    format: metal::MTLPixelFormat,
    load: metal::MTLLoadAction,
    store: metal::MTLStoreAction,
}

impl RenderPass {
    pub(super) fn num_color_attachments(&self) -> usize {
        self.data.colors.len()
    }

    pub(super) fn color_format(
        &self,
        _subpass: base::SubpassIndex,
        index: base::RenderSubpassColorTargetIndex,
    ) -> metal::MTLPixelFormat {
        self.data.colors[index]
            .as_ref()
            .map(|target| target.format)
            .unwrap_or(metal::MTLPixelFormat::Invalid)
    }

    pub(super) fn depth_format(&self, _subpass: base::SubpassIndex) -> metal::MTLPixelFormat {
        self.data
            .depth
            .as_ref()
            .map(|target| target.format)
            .unwrap_or(metal::MTLPixelFormat::Invalid)
    }

    pub(super) fn stencil_format(&self, _subpass: base::SubpassIndex) -> metal::MTLPixelFormat {
        self.data
            .stencil
            .as_ref()
            .map(|target| target.format)
            .unwrap_or(metal::MTLPixelFormat::Invalid)
    }
}

/// Implementation of `RenderTargetTableBuilder` for Metal.
#[derive(Debug, Clone)]
pub struct RenderTargetTableBuilder {
    /// A reference to a `MTLDevice`. We are not required to maintain a strong
    /// reference. (See the base interface's documentation)
    metal_device: metal::MTLDevice,
    label: Option<String>,

    render_pass: Option<RenderPass>,
    extents: Option<[u32; 2]>,
    num_layers: u32,
    targets: Vec<Option<Target>>,
}

zangfx_impl_object! { RenderTargetTableBuilder: base::RenderTargetTableBuilder, ::Debug }

unsafe impl Send for RenderTargetTableBuilder {}
unsafe impl Sync for RenderTargetTableBuilder {}

/// Implementation of `RenderTarget` for Metal.
#[derive(Debug, Clone)]
struct Target {
    image: Image,
    mip_level: u32,
    layer: u32,
    clear_color: metal::MTLClearColor,
    clear_depth: f32,
    clear_stencil: u32,
}

zangfx_impl_object! { Target: base::RenderTarget, ::Debug }

impl RenderTargetTableBuilder {
    /// Construct a `RenderTargetTableBuilder`.
    ///
    /// Ir's up to the caller to maintain the lifetime of `metal_device`.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device,
            label: None,

            render_pass: None,
            extents: None,
            num_layers: 1,
            targets: Vec::new(),
        }
    }
}

impl base::RenderTargetTableBuilder for RenderTargetTableBuilder {
    fn render_pass(&mut self, v: &base::RenderPass) -> &mut base::RenderTargetTableBuilder {
        let our_rp: &RenderPass = v.downcast_ref().expect("bad render pass type");
        self.render_pass = Some(our_rp.clone());
        self
    }

    fn extents(&mut self, v: &[u32]) -> &mut base::RenderTargetTableBuilder {
        self.extents = Some([
            v.get(0).cloned().unwrap_or(1),
            v.get(1).cloned().unwrap_or(1),
        ]);
        self
    }

    fn num_layers(&mut self, v: u32) -> &mut base::RenderTargetTableBuilder {
        self.num_layers = v;
        self
    }

    fn target(
        &mut self,
        index: base::RenderPassTargetIndex,
        view: &base::Image,
    ) -> &mut base::RenderTarget {
        if self.targets.len() <= index {
            self.targets.resize(index + 1, None);
        }

        let our_image: &Image = view.downcast_ref().expect("bad image type");
        self.targets[index] = Some(Target {
            image: our_image.clone(),
            mip_level: 0,
            layer: 0,
            clear_color: metal::MTLClearColor::new(0.0, 0.0, 0.0, 0.0),
            clear_depth: 0.0,
            clear_stencil: 0,
        });

        self.targets[index].as_mut().unwrap()
    }

    fn build(&mut self) -> Result<base::RenderTargetTable> {
        let render_pass: RenderPass = self.render_pass
            .clone()
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "render_pass"))?;
        let extents = self.extents
            .ok_or_else(|| Error::with_detail(ErrorKind::InvalidUsage, "extents"))?;

        let metal_desc = OCPtr::new(metal::MTLRenderPassDescriptor::new())
            .ok_or_else(|| nil_error("MTLRenderPassDescriptor renderPassDescriptor"))?;

        let populate_attachment_descriptor =
            |metal_desc: metal::MTLRenderPassAttachmentDescriptor, pass_target: &PassTarget| {
                metal_desc.set_load_action(pass_target.load);
                metal_desc.set_store_action(pass_target.store);

                let target: &Target = self.targets[pass_target.index].as_ref().unwrap();

                metal_desc.set_texture(target.image.metal_texture());
                metal_desc.set_level(target.mip_level as u64);
                metal_desc.set_slice(target.layer as u64);

                target
            };

        for (i, pass_color_target) in render_pass.data.colors.iter().enumerate() {
            if let &Some(ref pass_color_target) = pass_color_target {
                let metal_att_desc = metal_desc.color_attachments().object_at(i);

                let target = populate_attachment_descriptor(*metal_att_desc, pass_color_target);
                metal_att_desc.set_clear_color(target.clear_color);
            }
        }

        if let Some(ref pass_depth_target) = render_pass.data.depth {
            let metal_att_desc = metal_desc.depth_attachment();

            let target = populate_attachment_descriptor(*metal_att_desc, pass_depth_target);
            metal_att_desc.set_clear_depth(target.clear_depth as f64);
        }

        if let Some(ref pass_stencil_target) = render_pass.data.stencil {
            let metal_att_desc = metal_desc.stencil_attachment();

            let target = populate_attachment_descriptor(*metal_att_desc, pass_stencil_target);
            metal_att_desc.set_clear_stencil(target.clear_stencil);
        }

        if self.num_layers > 1 {
            metal_desc.set_render_target_array_length(self.num_layers as u64);
        }

        Ok(RenderTargetTable {
            metal_render_pass: metal_desc,
            extents,
        }.into())
    }
}

impl base::RenderTarget for Target {
    fn mip_level(&mut self, v: u32) -> &mut base::RenderTarget {
        self.mip_level = v;
        self
    }

    fn layer(&mut self, v: u32) -> &mut base::RenderTarget {
        self.layer = v;
        self
    }

    fn clear_float(&mut self, v: &[f32]) -> &mut base::RenderTarget {
        let v = &v[0..4];
        self.clear_color =
            metal::MTLClearColor::new(v[0] as f64, v[1] as f64, v[2] as f64, v[3] as f64);
        self
    }

    fn clear_uint(&mut self, v: &[u32]) -> &mut base::RenderTarget {
        let v = &v[0..4];
        self.clear_color =
            metal::MTLClearColor::new(v[0] as f64, v[1] as f64, v[2] as f64, v[3] as f64);
        self
    }

    fn clear_sint(&mut self, v: &[i32]) -> &mut base::RenderTarget {
        let v = &v[0..4];
        self.clear_color =
            metal::MTLClearColor::new(v[0] as f64, v[1] as f64, v[2] as f64, v[3] as f64);
        self
    }

    fn clear_depth_stencil(&mut self, depth: f32, stencil: u32) -> &mut base::RenderTarget {
        self.clear_depth = depth;
        self.clear_stencil = stencil;
        self
    }
}

/// Implementation of `RenderTargetTable` for Metal.
#[derive(Debug, Clone)]
pub struct RenderTargetTable {
    metal_render_pass: OCPtr<metal::MTLRenderPassDescriptor>,
    extents: [u32; 2],
}

zangfx_impl_handle! { RenderTargetTable, base::RenderTargetTable }

unsafe impl Send for RenderTargetTable {}
unsafe impl Sync for RenderTargetTable {}

impl RenderTargetTable {
    pub fn metal_render_pass(&self) -> metal::MTLRenderPassDescriptor {
        *self.metal_render_pass
    }

    pub(crate) fn extents(&self) -> [u32; 2] {
        self.extents
    }
}
