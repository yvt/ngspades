//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
pub mod di {
    use injector::{prelude::*, Container};
    use std::sync::Arc;

    use super::*;
    use crate::{
        di::DeviceContainer, spawner::di::SpawnerDeviceContainerExt,
        staticdata::di::StaticDataDeviceContainerExt,
    };

    pub trait TestPassRendererDeviceContainerExt {
        fn get_test_pass_renderer_or_build(&mut self) -> &gfx::Result<Arc<TestPassRenderer>>;
        fn register_test_pass_renderer_default(&mut self);
    }

    impl TestPassRendererDeviceContainerExt for Container {
        fn get_test_pass_renderer_or_build(&mut self) -> &gfx::Result<Arc<TestPassRenderer>> {
            self.get_singleton_or_build().unwrap()
        }

        fn register_test_pass_renderer_default(&mut self) {
            self.register_singleton_factory(|container| {
                let device = container.get_device().clone();
                let spawner = container.get_spawner_or_build().clone();
                let vertices = container.get_huge_triangle_vertices_or_build().clone();

                TestPassRenderer::new(device, spawner, vertices).map(Arc::new)
            });
        }
    }
}

use asynclazy::Async;
use cgmath::Vector2;
use include_data::{include_data, DataView};
use std::sync::Arc;
use zangfx::{base as gfx, prelude::*};

use crate::{spawner::Spawner, staticdata::StaticBuffer};
use ngsgamegfx_common::progress::Progress;
use ngsgamegfx_graph::passman::{
    ImageResource, ImageResourceInfo, Pass, PassEncodingContext, PassInfo, ResourceRef,
    ScheduleBuilder,
};

static SPIRV_FRAG: DataView = include_data!(concat!(env!("OUT_DIR"), "/testpass.frag.spv"));
static SPIRV_VERT: DataView = include_data!(concat!(env!("OUT_DIR"), "/testpass.vert.spv"));

const RT_FORMAT: gfx::ImageFormat = gfx::ImageFormat::SrgbRgba8;

const VERTEX_BUFFER_MAIN: gfx::VertexBufferIndex = 0;
const VERTEX_ATTR_POSITION: gfx::VertexAttrIndex = 0;

#[derive(Debug)]
pub struct TestPassRenderer {
    device: gfx::DeviceRef,
    vertices: Arc<StaticBuffer>,
    pipeline: Async<gfx::Result<gfx::RenderPipelineRef>>,
    render_pass: gfx::RenderPassRef,
}

impl TestPassRenderer {
    fn new(
        device: gfx::DeviceRef,
        spawner: Arc<dyn Spawner>,
        vertices: Arc<StaticBuffer>,
    ) -> gfx::Result<Self> {
        let mut spawn = spawner.get_spawn();

        let render_pass = {
            let mut builder = device.build_render_pass();
            builder
                .target(0)
                .set_format(RT_FORMAT)
                .set_load_op(gfx::LoadOp::DontCare)
                .set_store_op(gfx::StoreOp::Store);
            builder.subpass_color_targets(&[Some(0)]);
            builder.label("TestPassRenderer");
            builder.build()?
        };

        let pipeline = Async::with_future(&mut *spawn, {
            let device = device.clone();
            let render_pass = render_pass.clone();
            async move {
                let vertex_shader = device.new_library(SPIRV_VERT.as_u32_slice()).unwrap();
                let fragment_shader = device.new_library(SPIRV_FRAG.as_u32_slice()).unwrap();

                let root_sig = device.build_root_sig().build().unwrap();

                let mut builder = device.build_render_pipeline();
                builder
                    .vertex_shader(&vertex_shader, "main")
                    .fragment_shader(&fragment_shader, "main")
                    .root_sig(&root_sig)
                    .topology(gfx::PrimitiveTopology::Triangles)
                    .render_pass(&render_pass, 0);
                builder.vertex_buffer(VERTEX_BUFFER_MAIN, 4);
                builder.vertex_attr(
                    VERTEX_ATTR_POSITION,
                    VERTEX_BUFFER_MAIN,
                    0,
                    <u16>::as_format_unnorm() * 2,
                );
                builder.rasterize();
                builder.build()
            }
        }).unwrap();

        Ok(Self {
            device,
            render_pass,
            pipeline,
            vertices,
        })
    }

    pub fn ready_state(&self) -> Progress {
        Progress::from(self.vertices.buffer().is_some()) +
        Progress::from(self.pipeline.try_get().is_some())
    }

    pub fn define_pass<C: ?Sized>(
        self: &Arc<Self>,
        schedule_builder: &mut ScheduleBuilder<C>,
        extents: Vector2<u32>,
    ) -> ResourceRef<ImageResourceInfo> {
        // Define the output
        let output_info = ImageResourceInfo::new(extents.into(), RT_FORMAT);
        let output = schedule_builder.define_resource(output_info);

        let renderer = Arc::clone(self);

        schedule_builder.define_pass(PassInfo {
            resource_uses: vec![output.use_as_producer()],
            factory: Box::new(move |_context| {
                // We assume that the output resource is late-bound. So we don't
                // create any GFX resources here.
                Ok(Box::new(TestPass {
                    renderer,
                    output,
                    extents,
                }))
            }),
        });

        output
    }
}

impl Drop for TestPassRenderer {
    fn drop(&mut self) {
        // Wait at least until the pipeline compilation completes
        self.pipeline.get();
    }
}

#[derive(Debug)]
struct TestPass {
    renderer: Arc<TestPassRenderer>,
    output: ResourceRef<ImageResourceInfo>,
    extents: Vector2<u32>,
}

impl<C: ?Sized> Pass<C> for TestPass {
    fn encode(
        &mut self,
        cmd_buffer: &mut gfx::CmdBufferRef,
        wait_fences: &[&gfx::FenceRef],
        update_fences: &[&gfx::FenceRef],
        _context: &C,
        enc_context: &PassEncodingContext,
    ) -> gfx::Result<()> {
        assert_eq!(update_fences.len(), 1);

        let device = &self.renderer.device;
        let output: &ImageResource = enc_context.get_resource(self.output);

        let vertex_buffer = self.renderer.vertices.buffer();
        let pipeline = self.renderer.pipeline.try_get();

        let viewport = gfx::Viewport {
            x: 0f32,
            y: 0f32,
            width: self.extents.x as f32,
            height: self.extents.y as f32,
            min_depth: 0f32,
            max_depth: 1f32,
        };

        let rtt = {
            let mut builder = device.build_render_target_table();
            builder.target(0, &output.image);
            builder
                .render_pass(&self.renderer.render_pass)
                .extents(&[self.extents.x, self.extents.y])
                .build()?
        };

        {
            let e = cmd_buffer.encode_render(&rtt);

            for fence in wait_fences {
                e.wait_fence(fence, gfx::AccessTypeFlags::COLOR_WRITE);
            }

            if let (Some(vb), Some(p)) = (vertex_buffer, pipeline) {
                // FIXME: `gfx::Error` is `!Clone`, so we can't use `? `on `&gfx::Result`
                let vb = vb.as_ref().expect("Failed to create a vertex buffer");

                // FIXME: `gfx::Error` is `!Clone`, so we can't use `? `on `&gfx::Result`
                let p = p.as_ref().expect("Failed to create a pipeline");

                e.bind_pipeline(p);
                e.bind_vertex_buffers(0, &[(vb, 0)]);
                e.set_viewports(0, &[viewport]);
                e.draw(0..3, 0..1);
            }

            e.update_fence(update_fences[0], gfx::AccessTypeFlags::COLOR_WRITE);
        }

        Ok(())
    }
}
