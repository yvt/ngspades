//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;

use std::sync::Mutex;

use {OCPtr, RefEqArc};
use imp::ImageView;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderPass {
    data: RefEqArc<RenderPassData>,
}

#[derive(Debug)]
struct RenderPassData {
    attachments: Vec<core::RenderPassAttachmentDescription>,
    subpasses: Vec<RenderSubpassData>,
    label: Mutex<Option<String>>,
}

#[derive(Debug)]
struct RenderSubpassData {
    color_attachments: Vec<Option<RenderSubpassAttachment>>,
    depth_attachment: Option<RenderSubpassAttachment>,
    stencil_attachment: Option<RenderSubpassAttachment>,
}

#[derive(Debug)]
struct RenderSubpassAttachment {
    index: usize,
    load_action: metal::MTLLoadAction,
    store_action: metal::MTLStoreAction,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum AttachmentBinding {
    Color(usize),
    Depth,
    Stencil,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum InputAttachmentBinding {
    Color(usize),
    DepthStencil,
}

impl AttachmentBinding {
    fn input_binding(&self) -> InputAttachmentBinding {
        match *self {
            AttachmentBinding::Color(index) => InputAttachmentBinding::Color(index),
            AttachmentBinding::Depth |
            AttachmentBinding::Stencil => InputAttachmentBinding::DepthStencil,
        }
    }
}

fn translate_load_op(load_op: core::AttachmentLoadOp) -> metal::MTLLoadAction {
    match load_op {
        core::AttachmentLoadOp::Load => metal::MTLLoadAction::Load,
        core::AttachmentLoadOp::DontCare => metal::MTLLoadAction::DontCare,
        core::AttachmentLoadOp::Clear => metal::MTLLoadAction::Clear,
    }
}

fn translate_store_op(store_op: core::AttachmentStoreOp) -> metal::MTLStoreAction {
    match store_op {
        core::AttachmentStoreOp::Store => metal::MTLStoreAction::Store,
        core::AttachmentStoreOp::DontCare => metal::MTLStoreAction::DontCare,
    }
}

impl RenderPass {
    pub(crate) fn new(description: &core::RenderPassDescription) -> Self {
        let attachments: &[core::RenderPassAttachmentDescription] = &description.attachments;
        let subpasses: &[core::RenderSubpassDescription] = &description.subpasses;

        let mut data = RenderPassData {
            attachments: attachments.to_vec(),
            subpasses: Vec::with_capacity(subpasses.len()),
            label: Mutex::new(None),
        };

        let mut attachment_last_use = vec![None; attachments.len()];

        for (i, sp) in subpasses.iter().enumerate() {
            let mut handle_attachment =
                |att_index_or_none, binding, data_subpasses: &mut [RenderSubpassData]| {
                    if let Some(att_index) = att_index_or_none {
                        // skip aspect that is not supported by the image format
                        let ref attachment: core::RenderPassAttachmentDescription = attachments[att_index];
                        let valid_aspect: bool = match binding {
                            AttachmentBinding::Color(_) => attachment.format.has_color(),
                            AttachmentBinding::Depth => attachment.format.has_depth(),
                            AttachmentBinding::Stencil => attachment.format.has_stencil(),
                        };
                        if !valid_aspect {
                            return None;
                        }

                        // track usage of the attachment
                        let ref mut last_use: Option<(usize, InputAttachmentBinding)> = attachment_last_use[att_index];
                        let input_binding: InputAttachmentBinding = binding.input_binding();

                        // update the store op of the last subpass that uses the attachment
                        match *last_use {
                            Some((last_subpass_index, last_binding)) if last_subpass_index == i => {
                                if input_binding != last_binding {
                                    panic!(
                                        "The attachment {} is used for more than once in the subpass {}",
                                        att_index,
                                        i
                                    );
                                }
                            }
                            Some((last_subpass_index,
                                  InputAttachmentBinding::Color(last_att_binding))) => {
                                let ref mut last_subpass = data_subpasses[last_subpass_index];
                                last_subpass.color_attachments[last_att_binding]
                                    .as_mut()
                                    .unwrap()
                                    .store_action = metal::MTLStoreAction::Store;
                            }
                            Some((last_subpass_index, InputAttachmentBinding::DepthStencil)) => {
                                let ref mut last_subpass = data_subpasses[last_subpass_index];
                                if let Some(a) = last_subpass.depth_attachment.as_mut() {
                                    a.store_action = metal::MTLStoreAction::Store;
                                }
                                if let Some(a) = last_subpass.stencil_attachment.as_mut() {
                                    a.store_action = metal::MTLStoreAction::Store;
                                }
                            }
                            None => {}
                        }

                        // load from an external pass or the last subpass that uses the attachment
                        let load_action = if *last_use == None {
                            if binding == AttachmentBinding::Stencil {
                                translate_load_op(attachment.stencil_load_op)
                            } else {
                                translate_load_op(attachment.load_op)
                            }
                        } else {
                            metal::MTLLoadAction::Load
                        };

                        // track the usage
                        *last_use = Some((i, input_binding));

                        Some(RenderSubpassAttachment {
                            index: att_index,
                            load_action,
                            store_action: metal::MTLStoreAction::Store, // set later
                        })
                    } else {
                        None
                    }
                };

            let new_subpass = RenderSubpassData {
                color_attachments: sp.color_attachments
                    .iter()
                    .enumerate()
                    .map(|e| {
                        handle_attachment(
                            e.1.attachment_index,
                            AttachmentBinding::Color(e.0),
                            &mut data.subpasses,
                        )
                    })
                    .collect(),
                depth_attachment: sp.depth_stencil_attachment.and_then(|att| {
                    handle_attachment(
                        att.attachment_index,
                        AttachmentBinding::Depth,
                        &mut data.subpasses,
                    )
                }),
                stencil_attachment: sp.depth_stencil_attachment.and_then(|att| {
                    handle_attachment(
                        att.attachment_index,
                        AttachmentBinding::Stencil,
                        &mut data.subpasses,
                    )
                }),
            };
            data.subpasses.push(new_subpass);
        }

        for (att_index, last_use) in attachment_last_use.iter().enumerate() {
            let ref attachment: core::RenderPassAttachmentDescription = attachments[att_index];
            match *last_use {
                Some((last_subpass_index, InputAttachmentBinding::Color(last_att_binding))) => {
                    let ref mut last_subpass = data.subpasses[last_subpass_index];
                    last_subpass.color_attachments[last_att_binding]
                        .as_mut()
                        .unwrap()
                        .store_action = translate_store_op(attachment.store_op);
                }
                Some((last_subpass_index, InputAttachmentBinding::DepthStencil)) => {
                    let ref mut last_subpass = data.subpasses[last_subpass_index];
                    if let Some(a) = last_subpass.depth_attachment.as_mut() {
                        a.store_action = translate_store_op(attachment.store_op);
                    }
                    if let Some(a) = last_subpass.stencil_attachment.as_mut() {
                        a.store_action = translate_store_op(attachment.stencil_store_op);
                    }
                }
                None => {}
            }
        }

        Self { data: RefEqArc::new(data) }
    }

    pub(crate) fn num_subpass_color_attachments(&self, subpass: usize) -> usize {
        self.data.subpasses[subpass].color_attachments.len()
    }

    pub(crate) fn subpass_color_attachment_format(
        &self,
        subpass: usize,
        index: usize,
    ) -> Option<core::ImageFormat> {
        self.data.subpasses[subpass].color_attachments[index]
            .as_ref()
            .map(|att| self.data.attachments[att.index].format)
    }

    pub(crate) fn subpass_depth_attachment_format(
        &self,
        subpass: usize,
    ) -> Option<core::ImageFormat> {
        self.data.subpasses[subpass].depth_attachment.as_ref().map(
            |a| {
                self.data.attachments[a.index].format
            },
        )
    }

    pub(crate) fn subpass_stencil_attachment_format(
        &self,
        subpass: usize,
    ) -> Option<core::ImageFormat> {
        self.data.subpasses[subpass]
            .stencil_attachment
            .as_ref()
            .map(|a| self.data.attachments[a.index].format)
    }
}

impl core::Marker for RenderPass {
    fn set_label(&self, label: Option<&str>) {
        *self.data.label.lock().unwrap() = label.map(String::from);
    }
}

impl core::RenderPass for RenderPass {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Framebuffer {
    data: RefEqArc<FramebufferData>,
}

#[derive(Debug)]
pub(crate) struct FramebufferData {
    metal_descriptors: Vec<OCPtr<metal::MTLRenderPassDescriptor>>,
    label: Mutex<Option<String>>,
}

unsafe impl Send for FramebufferData {}
unsafe impl Sync for FramebufferData {} // no interior mutability

impl Framebuffer {
    pub(crate) fn new(description: &core::FramebufferDescription<RenderPass, ImageView>) -> Self {
        let ref render_pass: RenderPassData = *description.render_pass.data;
        assert_eq!(render_pass.attachments.len(), description.attachments.len());

        let populate_attachment_descriptor =
            |descriptor: metal::MTLRenderPassAttachmentDescriptor,
             attachment_info: &RenderSubpassAttachment| {
                descriptor.set_load_action(attachment_info.load_action);
                descriptor.set_store_action(attachment_info.store_action);

                let ref fb_att_desc = description.attachments[attachment_info.index];
                let iv = fb_att_desc.image_view;

                let (metal_texture, range) = iv.metal_texture_with_range();
                debug_assert_eq!(range.num_mip_levels, 1);
                debug_assert_eq!(range.num_array_layers, 1);
                descriptor.set_texture(metal_texture);
                descriptor.set_level(range.base_mip_level as u64);
                descriptor.set_slice(range.base_array_layer as u64);
            };

        let metal_descriptors: Vec<OCPtr<metal::MTLRenderPassDescriptor>> = render_pass
            .subpasses
            .iter()
            .map(|subpass| {
                let metal_descriptor = OCPtr::new(metal::MTLRenderPassDescriptor::new()).unwrap();

                for (i, att_or_none) in subpass.color_attachments.iter().enumerate() {
                    if let Some(att) = att_or_none.as_ref() {
                        let att_descriptor = metal_descriptor.color_attachments().object_at(i);
                        populate_attachment_descriptor(*att_descriptor, att);

                        let clear_value = match description.attachments[att.index].clear_values {
                            core::ClearValues::ColorFloat(values) => {
                                [
                                    values[0] as f64,
                                    values[1] as f64,
                                    values[2] as f64,
                                    values[3] as f64,
                                ]
                            }
                            core::ClearValues::ColorUnsignedInteger(values) => {
                                [
                                    values[0] as f64,
                                    values[1] as f64,
                                    values[2] as f64,
                                    values[3] as f64,
                                ]
                            }
                            core::ClearValues::ColorSignedInteger(values) => {
                                [
                                    values[0] as f64,
                                    values[1] as f64,
                                    values[2] as f64,
                                    values[3] as f64,
                                ]
                            }
                            core::ClearValues::DepthStencil(_, _) => {
                                panic!("invalid clear value for color attachment")
                            }
                        };
                        att_descriptor.set_clear_color(metal::MTLClearColor::new(
                            clear_value[0],
                            clear_value[1],
                            clear_value[2],
                            clear_value[3],
                        ));
                    }
                }

                if let Some(att) = subpass.depth_attachment.as_ref() {
                    populate_attachment_descriptor(*metal_descriptor.depth_attachment(), att);

                    let clear_value = match description.attachments[att.index].clear_values {
                        core::ClearValues::DepthStencil(depth, _) => depth,
                        _ => panic!("invalid clear value for color attachment"),
                    };
                    metal_descriptor.depth_attachment().set_clear_depth(
                        clear_value as f64,
                    );
                }

                if let Some(att) = subpass.stencil_attachment.as_ref() {
                    populate_attachment_descriptor(*metal_descriptor.stencil_attachment(), att);

                    let clear_value = match description.attachments[att.index].clear_values {
                        core::ClearValues::DepthStencil(_, stencil) => stencil,
                        _ => panic!("invalid clear value for color attachment"),
                    };
                    metal_descriptor.stencil_attachment().set_clear_stencil(
                        clear_value,
                    );
                }

                metal_descriptor
            })
            .collect();

        let data = FramebufferData {
            metal_descriptors,
            label: Mutex::new(None),
        };

        Self { data: RefEqArc::new(data) }
    }

    pub(crate) fn num_subpasses(&self) -> usize {
        self.data.metal_descriptors.len()
    }

    pub(crate) fn subpass(&self, index: usize) -> metal::MTLRenderPassDescriptor {
        *self.data.metal_descriptors[index]
    }
}

impl core::Marker for Framebuffer {
    fn set_label(&self, label: Option<&str>) {
        *self.data.label.lock().unwrap() = label.map(String::from);
    }
}

impl core::Framebuffer for Framebuffer {}
