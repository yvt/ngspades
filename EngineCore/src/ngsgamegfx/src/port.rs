//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a NgsPF port type for embedding a NgsGameGFX viewport.
use cgmath::{vec2, Vector2};
use futures::executor::block_on;
use std::{collections::VecDeque, sync::Arc};

use ngspf::core::{
    Context, KeyedProperty, KeyedPropertyAccessor, PresenterFrame, PropertyAccessor,
};
use ngspf::viewport;
#[allow(unused_imports)]
use zangfx::{base as gfx, prelude::*, utils as gfxut};

use crate::{
    config::Config,
    di::{new_device_container, CmdQueueSet},
    testpass::TestPassRenderer,
};
use ngsgamegfx_graph::{
    cbtasks::{CmdBufferTaskBuilder, CmdBufferTaskCellSet},
    passman::{ImageResource, ImageResourceInfo, ResourceRef},
    taskman::{Graph, GraphBuilder},
};

/// `Port` used to display the viewport of NgsGameGFX.
#[derive(Debug, Clone)]
pub struct PortRef(Arc<PortProps>);

impl PortRef {
    pub fn new(context: &Context) -> Self {
        PortRef(Arc::new(PortProps::new(context)))
    }

    pub fn config<'a>(&'a self) -> impl PropertyAccessor<Config> + 'a {
        fn select(this: &Arc<PortProps>) -> &KeyedProperty<Config> {
            &this.config
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }

    pub fn extents<'a>(&'a self) -> impl PropertyAccessor<Vector2<u32>> + 'a {
        fn select(this: &Arc<PortProps>) -> &KeyedProperty<Vector2<u32>> {
            &this.extents
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }
}

impl viewport::Port for PortRef {
    fn mount(&self, objects: &viewport::GfxObjects) -> Box<dyn viewport::PortInstance> {
        fn convert_gfx_queue(x: viewport::GfxQueue) -> (gfx::CmdQueueRef, gfx::QueueFamily) {
            (x.queue, x.queue_family)
        }

        // TODO: Share device DI container between port instances
        let mut device_container = new_device_container(
            objects.device.clone(),
            CmdQueueSet {
                main_queue: convert_gfx_queue(objects.main_queue.clone()),
                copy_queue: objects.copy_queue.clone().map(convert_gfx_queue),
            },
        );

        // Load the test renderer
        //
        // TODO: Render an actual content
        use crate::testpass::di::TestPassRendererDeviceContainerExt;
        let renderer = device_container
            .get_test_pass_renderer_or_build()
            .as_ref()
            .expect("Failed to create TestPassRenderer.")
            .clone();

        Box::new(Port {
            props: self.0.clone(),
            gfx_objects: objects.clone(),
            render_graph: None,
            renderer,
        })
    }
}

#[derive(Debug)]
struct PortProps {
    config: KeyedProperty<Config>,
    extents: KeyedProperty<Vector2<u32>>,
}

impl PortProps {
    pub fn new(context: &Context) -> Self {
        Self {
            config: KeyedProperty::new(context, Config::default()),
            extents: KeyedProperty::new(context, vec2(1280, 720)),
        }
    }
}

/// The implementation of `PortInstance`.
#[derive(Debug)]
struct Port {
    props: Arc<PortProps>,
    gfx_objects: viewport::GfxObjects,
    renderer: Arc<TestPassRenderer>,
    render_graph: Option<PortRenderGraph>,
}

#[derive(Debug)]
struct PortRenderGraph {
    /// The extents of the render target.
    extents: Vector2<u32>,

    /// CPU task graph.
    graph: Graph<gfx::Error>,

    /// I/O cells of command buffer generation/submission tasks
    /// defined in `graph`.
    cbtasks_cells: CmdBufferTaskCellSet,

    output_resource: ResourceRef<ImageResourceInfo>,

    /// `Future`s representing results of command buffer execution
    cb_results: VecDeque<gfxut::CmdBufferResult>,
}

#[derive(Debug)]
struct PortFrame<'a> {
    instance: &'a mut Port,
    frame: &'a PresenterFrame,
}

impl viewport::PortInstance for Port {
    fn start_frame<'a>(
        &'a mut self,
        frame: &'a PresenterFrame,
    ) -> gfx::Result<Box<dyn viewport::PortFrame + 'a>> {
        Ok(Box::new(PortFrame {
            instance: self,
            frame,
        }))
    }
}

impl viewport::PortFrame for PortFrame<'_> {
    fn image_extents(&mut self) -> [u32; 2] {
        (self.instance.props.extents)
            .read_presenter(self.frame)
            .unwrap()
            .clone()
            .into()
    }

    fn render(&mut self, context: &mut viewport::PortRenderContext) -> gfx::Result<()> {
        let instance = &mut *self.instance;

        let extents: Vector2<u32> = context.image_props.extents.into();
        let old_extents = instance.render_graph.as_ref().map(|g| g.extents);

        if Some(extents) != old_extents {
            // The extents of the viewport has changed. Re-create the render pass graph.
            instance.render_graph = None;
            instance.render_graph = Some(PortRenderGraph::new(
                &instance.gfx_objects,
                &instance.renderer,
                extents,
            )?);
        }

        let render_graph = instance.render_graph.as_mut().unwrap();
        render_graph.encode(context)?;

        context.schedule_next_frame = true;
        Ok(())
    }
}

impl PortRenderGraph {
    fn new(
        gfx_objects: &viewport::GfxObjects,
        renderer: &Arc<TestPassRenderer>,
        extents: Vector2<u32>,
    ) -> gfx::Result<Self> {
        let mut cb_task_builder = CmdBufferTaskBuilder::new();

        // Construct a GPU pass graph
        let output_resource = renderer.define_pass(cb_task_builder.schedule_builder(), extents);

        // The output image is supplied by the compositor, so mark it
        // as late-bound
        cb_task_builder
            .schedule_builder()
            .mark_resource_as_late_bound(&output_resource);

        let mut graph_builder = GraphBuilder::new();
        let cbtasks_cells = cb_task_builder.add_to_graph(
            &gfx_objects.device,
            &gfx_objects.main_queue.queue,
            &mut graph_builder,
            &[&output_resource],
            1, // num_result_cells
        )?;

        Ok(Self {
            extents,
            graph: graph_builder.build(),
            cbtasks_cells,
            cb_results: VecDeque::new(),
            output_resource,
        })
    }

    fn encode(&mut self, context: &mut viewport::PortRenderContext) -> gfx::Result<()> {
        // Retire old CBs
        while self.cb_results.len() > 2 {
            let cb_result = self.cb_results.pop_front().unwrap();
            block_on(cb_result).unwrap()?;
        }

        let graph = &mut self.graph;
        let cbtasks_cells = &self.cbtasks_cells;

        // Set graph inputs
        *graph.borrow_cell_mut(cbtasks_cells.update_fence) = Some(context.fence.clone());

        let image = context.image.clone();
        let output_resource = self.output_resource;
        graph
            .borrow_cell_mut(cbtasks_cells.late_resource_binder)
            .set(move |run| {
                run.bind(output_resource, ImageResource::new(image, None));
            });

        // Execute the task graph
        // FIXME: Should use the`UserInteractive` QoS class
        // (`High` is mapped to `UserInitiated`)
        let executor = xdispatch::Queue::global(xdispatch::QueuePriority::High);
        graph.run(&executor)?;

        // Get graph outputs
        let cb_result = graph
            .borrow_cell_mut(cbtasks_cells.cmd_buffer_results[0])
            .take()
            .unwrap();
        self.cb_results.push_back(cb_result);

        Ok(())
    }
}

impl Drop for PortRenderGraph {
    fn drop(&mut self) {
        for cb_result in self.cb_results.drain(..) {
            let _ = block_on(cb_result).unwrap();
        }
    }
}
