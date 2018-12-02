//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ngspf::viewport::zangfx::base as gfx;
use ngspf::viewport::zangfx::utils as gfxut;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use cgmath::prelude::*;
use cgmath::{vec3, Matrix4, Point2};

use refeq::RefEqArc;

use ngspf::cggeom::prelude::*;
use ngspf::cggeom::Box2;
use ngspf::prelude::*;
use ngspf::viewport::{
    LayerBuilder, LayerContents, RootRef, VirtualKeyCode, WindowBuilder, WindowEvent, WindowFlags,
    WindowRef, WorkspaceBuilder,
};

mod triangle {
    use include_data::{include_data, DataView};

    static SPIRV_FRAG: DataView = include_data!(concat!(env!("OUT_DIR"), "/triangle.frag.spv"));
    static SPIRV_VERT: DataView = include_data!(concat!(env!("OUT_DIR"), "/triangle.vert.spv"));

    use crate::gfx;
    use crate::gfx::prelude::*;
    use crate::gfxut::DeviceUtils;

    use flags_macro::flags;

    use std::mem;
    use std::sync::Arc;

    use ngspf::core::{
        Context, KeyedProperty, KeyedPropertyAccessor, PresenterFrame, PropertyAccessor,
    };
    use ngspf::viewport::{GfxObjects, GfxQueue, Port, PortFrame, PortInstance, PortRenderContext};

    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    struct Vertex {
        position: [f32; 3],
        color: [f32; 3],
    }

    const VERTEX_BUFFER_MAIN: gfx::VertexBufferIndex = 0;

    const VERTEX_ATTR_POSITION: gfx::VertexAttrIndex = 0;
    const VERTEX_ATTR_COLOR: gfx::VertexAttrIndex = 1;

    const RT_FORMAT: gfx::ImageFormat = gfx::ImageFormat::SrgbRgba8;

    #[derive(Debug, Clone)]
    pub struct MyPort {
        data: Arc<PortData>,
    }

    #[derive(Debug)]
    struct PortData {
        frame: KeyedProperty<u16>,
    }

    impl MyPort {
        pub fn new(context: &Context) -> Self {
            Self {
                data: Arc::new(PortData {
                    frame: KeyedProperty::new(context, 0),
                }),
            }
        }

        pub fn frame<'a>(&'a self) -> impl PropertyAccessor<u16> + 'a {
            fn select(this: &Arc<PortData>) -> &KeyedProperty<u16> {
                &this.frame
            }
            KeyedPropertyAccessor::new(&self.data, select)
        }
    }

    impl Port for MyPort {
        fn mount(&self, objects: &GfxObjects) -> Box<PortInstance> {
            Box::new(MyPortInstance::new(objects, self.data.clone()))
        }
    }

    #[derive(Debug)]
    struct MyPortInstance {
        device: gfx::DeviceRef,
        data: Arc<PortData>,
        main_queue: GfxQueue,
        vertex_buffer: gfx::BufferRef,
        pipeline: gfx::RenderPipelineRef,
        render_pass: gfx::RenderPassRef,
    }

    impl MyPortInstance {
        fn new(gfx_objects: &GfxObjects, data: Arc<PortData>) -> Self {
            let device = gfx_objects.device.clone();
            let main_queue = gfx_objects.main_queue.clone();

            let vertex_buffer = Self::make_vertex_buffer(&device);

            let render_pass = {
                let mut builder = device.build_render_pass();
                builder
                    .target(0)
                    .set_format(RT_FORMAT)
                    .set_load_op(gfx::LoadOp::Clear)
                    .set_store_op(gfx::StoreOp::Store);
                builder.subpass_color_targets(&[Some(0)]);
                builder.label("Port render pass");
                builder.build().unwrap()
            };

            let pipeline = Self::make_pipeline(&*device, &render_pass);

            Self {
                device,
                data,
                main_queue,
                vertex_buffer,
                pipeline,
                render_pass,
            }
        }

        fn make_pipeline(
            device: &gfx::Device,
            render_pass: &gfx::RenderPassRef,
        ) -> gfx::RenderPipelineRef {
            let vertex_shader = device.new_library(SPIRV_VERT.as_u32_slice()).unwrap();
            let fragment_shader = device.new_library(SPIRV_FRAG.as_u32_slice()).unwrap();

            let root_sig = device.build_root_sig().build().unwrap();

            let mut builder = device.build_render_pipeline();
            builder
                .vertex_shader(&vertex_shader, "main")
                .fragment_shader(&fragment_shader, "main")
                .root_sig(&root_sig)
                .topology(gfx::PrimitiveTopology::Triangles)
                .render_pass(render_pass, 0);
            builder.vertex_buffer(VERTEX_BUFFER_MAIN, mem::size_of::<Vertex>() as _);
            builder.vertex_attr(
                VERTEX_ATTR_POSITION,
                VERTEX_BUFFER_MAIN,
                0,
                <f32>::as_format() * 3,
            );
            builder.vertex_attr(
                VERTEX_ATTR_COLOR,
                VERTEX_BUFFER_MAIN,
                12,
                <f32>::as_format() * 3,
            );
            builder.rasterize();
            builder.build().unwrap()
        }

        fn make_vertex_buffer(device: &gfx::DeviceRef) -> gfx::BufferRef {
            let vertices = [
                Vertex {
                    position: [-0.5f32, 0.5f32, 0f32],
                    color: [1f32, 0f32, 0f32],
                },
                Vertex {
                    position: [0.5f32, 0.5f32, 0f32],
                    color: [0f32, 1f32, 0f32],
                },
                Vertex {
                    position: [0f32, -0.5f32, 0f32],
                    color: [0f32, 0f32, 1f32],
                },
            ];

            use std::mem::size_of_val;
            use std::slice::from_raw_parts_mut;

            let size = size_of_val(&vertices);

            let buffer = device
                .build_buffer()
                .size(size as u64)
                .usage(gfx::BufferUsageFlags::Vertex)
                .build()
                .unwrap();

            let memory_type = device
                .choose_memory_type(
                    buffer.get_memory_req().unwrap().memory_types,
                    flags![gfx::MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
                    flags![gfx::MemoryTypeCapsFlags::{HostVisible | HostCoherent}],
                )
                .unwrap();

            let success = device
                .global_heap(memory_type)
                .bind((&buffer).into())
                .unwrap();
            assert!(success);

            let slice: &mut [Vertex] =
                unsafe { from_raw_parts_mut(buffer.as_ptr() as *mut Vertex, vertices.len()) };
            slice.copy_from_slice(&vertices);

            buffer
        }
    }

    impl PortInstance for MyPortInstance {
        fn start_frame<'a>(
            &'a mut self,
            frame: &'a PresenterFrame,
        ) -> gfx::Result<Box<dyn PortFrame + 'a>> {
            Ok(Box::new(MyPortFrame {
                instance: self,
                frame,
            }))
        }
    }

    #[derive(Debug)]
    struct MyPortFrame<'a> {
        instance: &'a mut MyPortInstance,
        frame: &'a PresenterFrame,
    }

    impl PortFrame for MyPortFrame<'_> {
        fn image_extents(&mut self) -> [u32; 2] {
            [128, 128]
        }

        fn render(&mut self, context: &mut PortRenderContext) -> gfx::Result<()> {
            let instance = &mut *self.instance;
            let frame = self.frame;
            let frame_index = *instance.data.frame.read_presenter(frame).unwrap() as u32;

            let ref extents = context.image_props.extents;
            assert_eq!(context.image_props.format, RT_FORMAT);

            let viewport = gfx::Viewport {
                x: 0f32,
                y: 0f32,
                width: extents[0] as f32,
                height: extents[1] as f32,
                min_depth: 0f32,
                max_depth: 1f32,
            };

            let rtt = {
                let mut builder = instance.device.build_render_target_table();
                builder
                    .target(0, &context.image)
                    .clear_float(&[0.2, 0.2, 0.2, 1.0]);
                builder
                    .render_pass(&instance.render_pass)
                    .extents(extents)
                    .build()?
            };

            let mut buffer = instance.main_queue.queue.new_cmd_buffer()?;
            {
                let e = buffer.encode_render(&rtt);
                e.bind_pipeline(&instance.pipeline);
                e.bind_vertex_buffers(0, &[(&instance.vertex_buffer, 0)]);
                e.set_viewports(0, &[viewport]);
                e.draw(0..3, frame_index..frame_index + 1); // easiest way to pass a number

                e.update_fence(&context.fence, gfx::AccessTypeFlags::ColorWrite);
            }
            buffer.commit().unwrap();

            context.schedule_next_frame = true;
            Ok(())
        }
    }
}

fn main() {
    let mut ws = WorkspaceBuilder::new()
        .application_name("NgsPF Example: port")
        .application_version(1, 0, 0)
        .build()
        .expect("failed to create a workspace");
    let context = Arc::clone(ws.context());
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);

    // Produce the first frame
    let root = RootRef::clone(ws.root());
    let window: WindowRef;
    let port = RefEqArc::new(crate::triangle::MyPort::new(&context));
    {
        let image = LayerBuilder::new()
            .contents(LayerContents::Port(port.clone()))
            .bounds(Box2::new(Point2::origin(), Point2::new(512.0, 512.0)))
            .transform(Matrix4::from_translation(vec3(10.0, 10.0, 0.0)))
            .build(&context);

        window = WindowBuilder::new()
            .flags(WindowFlags::Resizable)
            .child(Some(image.into_node_ref()))
            .listener(Some(Box::new(move |event| {
                // Send the event to the producer loop
                let _ = tx.lock().unwrap().send(event.clone());
            })))
            .build(&context);

        let mut frame = context
            .lock_producer_frame()
            .expect("failed to acquire a producer frame");
        ws.root()
            .windows()
            .set(&mut frame, Some(window.clone().into_node_ref()))
            .expect("failed to set the value of proeprty 'windows'");
    }
    context.commit().expect("failed to commit a frame");

    // Start the producer loop
    thread::Builder::new()
        .spawn(move || {
            use std::time::Duration;
            let mut i = 0;
            let mut exit = false;
            while !exit {
                i += 1;

                // Process window events
                for event in rx.try_iter() {
                    match event {
                        WindowEvent::Close => {
                            exit = true;
                        }
                        WindowEvent::KeyboardInput(vk, pressed, _) => {
                            if pressed && vk == VirtualKeyCode::Escape {
                                exit = true;
                            }
                        }
                        _ => {}
                    }
                }

                {
                    let mut frame = context
                        .lock_producer_frame()
                        .expect("failed to acquire a producer frame");

                    port.frame()
                        .set(&mut frame, ((i * 100) & 0xffff) as u16)
                        .unwrap();

                    if exit {
                        root.exit_loop(&mut frame).unwrap();
                    }
                }
                context.commit().expect("failed to commit a frame");
                thread::sleep(Duration::from_millis(15));
            }
        })
        .unwrap();

    // Start the main loop
    ws.enter_main_loop()
        .expect("error occured while running the main loop");
}
