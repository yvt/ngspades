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

use super::{RenderPass, ImageView};

pub trait Framebuffer: Hash + Debug + Clone + Eq + PartialEq + Send + Sync + Any {}

#[derive(Debug, Clone, Copy)]
pub struct FramebufferDescription<'a, TRenderPass: RenderPass, TImageView: ImageView> {
    pub render_pass: &'a TRenderPass,
    pub attachments: &'a [FramebufferAttachmentDescription<'a, TImageView>],
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct FramebufferAttachmentDescription<'a, TImageView: ImageView> {
    pub attachment: &'a TImageView,
    pub clear_values: ClearValues,
}

#[derive(Debug, Clone, Copy)]
pub struct ClearValues {
    /// Clear color values for normalized and floating point color images.
    pub clear_color_float: [f32; 4],

    /// Clear color values for unnormalized color images.
    pub clear_color_int: [u32; 4],

    /// Clear color values for depth images.
    pub clear_depth: f32,

    /// Clear color values for stencil images.
    pub clear_stencil: u32,
}

impl ::std::default::Default for ClearValues {
    fn default() -> Self {
        Self {
            clear_color_float: [0f32; 4],
            clear_color_int: [0u32; 4],
            clear_depth: 1f32,
            clear_stencil: 0,
        }
    }
}