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

use {ImageLayout, ImageFormat, PipelineStageFlags, AccessTypeFlags, Validate, DeviceCapabilities,
     Marker, SubpassIndex};

/// Handle for render pass objects.
pub trait RenderPass
    : Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any + Marker {
}

#[derive(Debug, Clone, Copy)]
pub struct RenderPassDescription<'a> {
    pub attachments: &'a [RenderPassAttachmentDescription],
    pub subpasses: &'a [RenderSubpassDescription<'a>],
    pub dependencies: &'a [RenderSubpassDependency],
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

/// Describes a render subpass.
///
/// See Vulkan 1.0 Specification "7.1. Render Pass Creation" for details.
/// Following items are not supported:
///
///  - Feedback loops.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderSubpassDescription<'a> {
    pub input_attachments: &'a [RenderPassAttachmentReference],
    pub color_attachments: &'a [RenderPassAttachmentReference],
    pub depth_stencil_attachment: Option<RenderPassAttachmentReference>,
    pub preserve_attachments: &'a [RenderPassAttachmentIndex],
}

#[derive(Debug, Clone, Copy)]
pub struct RenderPassAttachmentReference {
    pub attachment_index: Option<RenderPassAttachmentIndex>,
    pub layout: ImageLayout,
}

pub type RenderPassAttachmentIndex = usize;

/// Describes a dependency between subpasses.
///
/// See Vulkan 1.0 Specification "7.1. Render Pass Creation" for details.
#[derive(Debug, Clone, Copy)]
pub struct RenderSubpassDependency {
    /// The first subpass in the dependency.
    ///
    /// If `source` and `destination` are both not equal to `External`, the inequality
    /// `source` < `destination` must be satifsied (self-dependency is prohibited).
    pub source: RenderSubpassDependencyTarget,

    /// The second subpass in the dependency.
    ///
    /// If `source` and `destination` are both not equal to `External`, the inequality
    /// `source` < `destination` must be satifsied (self-dependency is prohibited).
    pub destination: RenderSubpassDependencyTarget,

    pub source_stage_mask: PipelineStageFlags,
    pub destination_stage_mask: PipelineStageFlags,
    pub source_access_mask: AccessTypeFlags,
    pub destination_access_mask: AccessTypeFlags,
    pub by_region: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderSubpassDependencyTarget {
    /// Specifies a subpass in the same render pass.
    ///
    /// `index` must be less than the number of subpasses (`RenderPassDescription::subpasses.len()`).
    Subpass { index: SubpassIndex },

    /// Specfiies all commands submitted to the queue before/after the render pass.
    External,
}

/// Validation errors for [`RenderPassDescription`](struct.RenderPassDescription.html).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum RenderPassDescriptionValidationError {
    // TODO
}

impl<'a> Validate for RenderPassDescription<'a> {
    type Error = RenderPassDescriptionValidationError;

    #[allow(unused_variables)]
    #[allow(unused_mut)]
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, mut callback: T)
    where
        T: FnMut(Self::Error) -> (),
    {
        // TODO
    }
}
