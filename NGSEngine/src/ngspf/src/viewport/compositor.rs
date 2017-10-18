//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::{Arc, Mutex};
use atomic_refcell::AtomicRefCell;
use gfx;
use gfx::core::Backend;
use gfx::prelude::*;
use context::{NodeRef, PresenterFrame};
use super::{WorkspaceDevice, Library};

#[derive(Debug)]
pub struct Compositor;

impl<B: Backend> Library<B> for Compositor {
    type LibraryId = ();
    type Instance = CompositorInstance<B>;

    fn id(&self) -> Self::LibraryId {
        ()
    }

    fn make_instance(&self, ws_device: &WorkspaceDevice<B>) -> Self::Instance {
        CompositorInstance {
            heap: Arc::clone(ws_device.objects().heap()),
            device: Arc::clone(ws_device.objects().gfx_device()),
            statesets: vec![
                Stateset::new(
                    &**ws_device.objects().gfx_device(),
                    gfx::core::ImageFormat::SrgbBgra8
                ).expect("Stateset creation failed"),
            ],
        }
    }
}

#[derive(Debug)]
pub struct CompositorInstance<B: Backend> {
    device: Arc<B::Device>,
    heap: Arc<Mutex<B::UniversalHeap>>,
    statesets: Vec<Stateset<B>>,
}

/// Pipeline states etc. specific to a framebuffer image format.
#[derive(Debug)]
struct Stateset<B: Backend> {
    framebuffer_format: gfx::core::ImageFormat,
    render_passes: Vec<B::RenderPass>,
}

const RENDER_PASS_BIT_CLEAR: usize = 1 << 0;
const RENDER_PASS_BIT_USAGE_MASK: usize = 0b11 << 1;
const RENDER_PASS_BIT_USAGE_PRESENT: usize = 0b00 << 1;
const RENDER_PASS_BIT_USAGE_SHADER_READ: usize = 0b01 << 1;
const RENDER_PASS_BIT_USAGE_TRANSFER: usize = 0b10 << 1;

#[derive(Debug)]
pub struct CompositorWindow<B: Backend> {
    compositor: Arc<CompositorInstance<B>>,
    command_buffer: Arc<AtomicRefCell<B::CommandBuffer>>,
}

#[derive(Debug)]
pub struct CompositeContext<'a, B: Backend> {
    pub workspace_device: &'a WorkspaceDevice<B>,
    pub schedule_next_frame: bool,
    /// Command buffers to be submitted to the device (after calls to `composite` are done).
    pub command_buffers: Vec<Arc<AtomicRefCell<B::CommandBuffer>>>,
}

impl<B: Backend> CompositorWindow<B> {
    pub fn new(compositor: Arc<CompositorInstance<B>>) -> Self {
        let command_buffer;
        {
            let ref device = compositor.device;
            command_buffer = device.main_queue().make_command_buffer().expect(
                "failed to create a command buffer",
            );
            command_buffer.set_label(Some("compositor main command buffer"));
        }
        Self {
            compositor,
            command_buffer: Arc::new(AtomicRefCell::new(command_buffer)),
        }
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
        _frame: &PresenterFrame,
        drawable: &D,
        drawable_info: &gfx::wsi::DrawableInfo,
    ) where
        D: gfx::wsi::Drawable<Backend = B>,
    {
        let device: &B::Device = context.workspace_device.objects().gfx_device();
        let image_view = device
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
        };

        let framebuffer = device
            .factory()
            .make_framebuffer(&gfx::core::FramebufferDescription {
                render_pass: &self.compositor.statesets[0].render_passes
                    [RENDER_PASS_BIT_CLEAR + RENDER_PASS_BIT_USAGE_PRESENT],
                attachments: &[
                    gfx::core::FramebufferAttachmentDescription {
                        image_view: &image_view,
                        clear_values: gfx::core::ClearValues::ColorFloat([0.5, 0.5, 0.5, 1.0]),
                    },
                ],
                width: drawable_info.extents.x,
                height: drawable_info.extents.y,
                num_layers: 1,
            })
            .unwrap();

        let cb_cell = Arc::clone(&self.command_buffer);
        {
            let mut cb = cb_cell.borrow_mut();
            cb.wait_completion().unwrap();
            cb.begin_encoding();
            cb.begin_render_pass(&framebuffer, gfx::core::DeviceEngine::Universal);
            {
                cb.begin_render_subpass(gfx::core::RenderPassContents::Inline);
                if let Some(fence) = drawable.acquiring_fence() {
                    cb.wait_fence(
                        gfx::core::PipelineStage::ColorAttachmentOutput.into(),
                        gfx::core::AccessType::ColorAttachmentWrite.into(),
                        fence,
                    );
                }
                cb.set_viewport(&viewport);
                // TODO
                if let Some(fence) = drawable.releasing_fence() {
                    cb.update_fence(
                        gfx::core::PipelineStage::ColorAttachmentOutput.into(),
                        gfx::core::AccessType::ColorAttachmentWrite.into(),
                        fence,
                    );
                }
                cb.end_render_subpass();
            }
            drawable.finalize(
                &mut cb,
                gfx::core::PipelineStage::ColorAttachmentOutput.into(),
                gfx::core::AccessType::ColorAttachmentWrite.into(),
                gfx::core::ImageLayout::Present,
            );
            cb.end_pass();
            cb.end_encoding().expect("command buffer encoding failed");
        }

        context.command_buffers.push(cb_cell);
    }
}

impl<B: Backend> Stateset<B> {
    fn new(
        device: &B::Device,
        framebuffer_format: gfx::core::ImageFormat,
    ) -> gfx::core::Result<Self> {
        let spb = gfx::core::RenderSubpassDescription {
            input_attachments: &[],
            color_attachments: &[
                gfx::core::RenderPassAttachmentReference {
                    attachment_index: Some(0),
                    layout: gfx::core::ImageLayout::ColorAttachment,
                },
            ],
            depth_stencil_attachment: None,
            preserve_attachments: &[],
        };

        let render_passes = (0..6)
            .map(|i| {
                let usage = i & RENDER_PASS_BIT_USAGE_MASK;

                let decs = gfx::core::RenderPassDescription {
                    attachments: &[
                        gfx::core::RenderPassAttachmentDescription {
                            may_alias: false,
                            format: framebuffer_format,
                            load_op: if (i & RENDER_PASS_BIT_CLEAR) != 0 {
                                gfx::core::AttachmentLoadOp::Clear
                            } else {
                                gfx::core::AttachmentLoadOp::Load
                            },
                            store_op: gfx::core::AttachmentStoreOp::Store,
                            stencil_load_op: gfx::core::AttachmentLoadOp::DontCare,
                            stencil_store_op: gfx::core::AttachmentStoreOp::DontCare,
                            initial_layout: if (i & RENDER_PASS_BIT_CLEAR) != 0 {
                                gfx::core::ImageLayout::Undefined
                            } else {
                                gfx::core::ImageLayout::TransferSource
                            },
                            final_layout: match usage {
                                RENDER_PASS_BIT_USAGE_PRESENT => gfx::core::ImageLayout::Present,
                                RENDER_PASS_BIT_USAGE_SHADER_READ => {
                                    gfx::core::ImageLayout::ShaderRead
                                }
                                RENDER_PASS_BIT_USAGE_TRANSFER => {
                                    gfx::core::ImageLayout::TransferSource
                                }
                                _ => unreachable!(),
                            },
                        },
                    ],
                    subpasses: &[spb],
                    dependencies: &[],
                };

                let render_pass = device.factory().make_render_pass(&decs);
                if let Ok(ref render_pass) = render_pass {
                    render_pass.set_label(Some("compositor render pass (clear)"));
                }
                render_pass
            })
            .collect::<gfx::core::Result<_>>()?;

        Ok(Self {
            framebuffer_format,
            render_passes,
        })
    }
}
