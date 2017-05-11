//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::clone::Clone;
use std::hash::Hash;
use std::fmt::Debug;
use std::cmp::{Eq, PartialEq};
use std::any::Any;

use enumflags::BitFlags;

use super::{ImageLayout, ImageFormat, PipelineStageFlags, AccessFlags};

/// Handle for render passes.
pub trait RenderPass: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}

#[derive(Debug, Clone, Copy)]
pub struct RenderPassDescription<'a> {
    pub attachments: &'a [RenderPassAttachmentDescription],
    pub subpasses: &'a [RenderSubpassDescription<'a>],
}

#[derive(Debug, Clone, Copy)]
pub struct RenderPassAttachmentDescription {
    pub may_alias: bool,
    pub format: ImageFormat,
    pub load_op: AttachmentLoadOp,
    pub store_op: AttachmentStoreOp,
    pub stencil_load_op: AttachmentLoadOp,
    pub stencil_store_op: AttachmentStoreOp,
    pub initial_layout: ImageLayout,
    pub final_layout: ImageLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttachmentLoadOp {
    Load,
    Clear,
    DontCare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttachmentStoreOp {
    Store,
    DontCare,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderSubpassDescription<'a> {
    pub input_attachments: &'a [RenderPassAttachmentReference],
    pub color_attachments: &'a [RenderPassAttachmentReference],
    pub depth_stencil_attachment: &'a RenderPassAttachmentReference,
    pub preserve_attachment_indices: &'a [usize],
}

#[derive(Debug, Clone, Copy)]
pub struct RenderPassAttachmentReference {
    pub attachment_index: usize,
    pub layout: ImageLayout,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderSubpassDependency {
    pub source: RenderSubpassDependencyTarget,
    pub destination: RenderSubpassDependencyTarget,
    pub source_stage_mask: BitFlags<PipelineStageFlags>,
    pub destination_stage_mask: BitFlags<PipelineStageFlags>,
    pub source_access_mask: BitFlags<AccessFlags>,
    pub destination_access_mask: BitFlags<AccessFlags>,
    pub by_region: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderSubpassDependencyTarget {
    Subpass { index: usize },
    External,
}
