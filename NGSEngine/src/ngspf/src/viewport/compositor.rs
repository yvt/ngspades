//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::{Arc, Mutex};
use gfx;
use gfx::core::Backend;
use gfx::prelude::*;
use context::NodeRef;
use super::{WorkspaceDevice, Library};

#[derive(Debug)]
pub struct Compositor;

impl<B: Backend> Library<B> for Compositor {
    type LibraryId = ();
    type Instance = CompositorInstance<B>;

    fn id(&self) -> Self::LibraryId {
        ()
    }

    fn make_instance(&self, device: &WorkspaceDevice<B>) -> Self::Instance {
        CompositorInstance {
            heap: Arc::clone(device.objects().heap()),
            device: Arc::clone(device.objects().gfx_device()),
        }
    }
}

#[derive(Debug)]
pub struct CompositorInstance<B: Backend> {
    device: Arc<B::Device>,
    heap: Arc<Mutex<B::UniversalHeap>>,
}

#[derive(Debug)]
pub struct CompositorWindow<B: Backend> {
    compositor: Arc<CompositorInstance<B>>,
}

#[derive(Debug)]
pub struct CompositeContext<'a, B: Backend> {
    pub workspace_device: &'a WorkspaceDevice<B>,
    pub schedule_next_frame: bool,
    /// Command buffers to be submitted to the device (after calls to `composite` are done).
    pub command_buffers: Vec<B::CommandBuffer>,
}

impl<B: Backend> CompositorWindow<B> {
    pub fn new(compositor: Arc<CompositorInstance<B>>) -> Self {
        Self { compositor }
    }

    pub fn frame_description(&self) -> gfx::wsi::FrameDescription {
        gfx::wsi::FrameDescription {
            acquiring_engines: gfx::core::DeviceEngine::Universal.into(),
            releasing_engines: gfx::core::DeviceEngine::Universal.into(),
        }
    }

    pub fn composite<D>(
        &mut self,
        context: &mut CompositeContext<B>,
        _root: &Option<NodeRef>,
        drawable: &D,
        _drawable_info: &gfx::wsi::DrawableInfo,
    ) where
        D: gfx::wsi::Drawable<Backend = B>,
    {
        let device: &B::Device = context.workspace_device.objects().gfx_device();
        /* let image_view = device
            .factory()
            .make_image_view(&gfx::core::ImageViewDescription {
                image_type: gfx::core::ImageType::TwoD,
                image: drawable.image(),
                format: drawable_info.format,
                range: gfx::core::ImageSubresourceRange::default(),
            })
            .unwrap();

        let viewport = gfx::core::Viewport {
            x: 0f32,
            y: 0f32,
            width: drawable_info.extents.x as f32,
            height: drawable_info.extents.y as f32,
            min_depth: 0f32,
            max_depth: 1f32,
        }; */

        let mut cb = device.main_queue().make_command_buffer().unwrap();
        cb.set_label(Some("compositor main command buffer"));

        cb.begin_encoding();
        // TODO
        cb.begin_copy_pass(gfx::core::DeviceEngine::Copy);
        drawable.finalize(
            &mut cb,
            gfx::core::PipelineStage::ColorAttachmentOutput.into(),
            gfx::core::AccessType::ColorAttachmentWrite.into(),
            gfx::core::ImageLayout::Present,
        );
        cb.end_pass();
        cb.end_encoding().expect("command buffer encoding failed");

        context.command_buffers.push(cb);
    }
}
