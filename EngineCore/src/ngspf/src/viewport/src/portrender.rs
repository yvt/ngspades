//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use refeq::RefEqArc;
use std::collections::HashMap;
use xdispatch;
use zangfx::{base as gfx, base::Result};

use core::{NodeRef, PresenterFrame};

use layer::{Layer, LayerContents};
use port::{GfxObjects, Port, PortImageProps, PortManager, PortRenderContext};
use temprespool::{TempResPool, TempResTable};

/// Manages rendering work on `Port`s in a certain frame.
#[derive(Debug)]
pub(crate) struct PortRenderFrame<'a> {
    outputs: HashMap<RefEqArc<Port>, PortRenderOutput>,
    queue: xdispatch::Queue,

    /// `PresenterFrame` must outlive `self`. Otherwise `PortInstance::render`
    /// would access the destroyed `PresenterFrame`.
    frame: &'a PresenterFrame,
}

#[derive(Debug)]
pub(crate) struct PortRenderOutput {
    pub image: gfx::ImageRef,
    pub fence: gfx::FenceRef,
}

impl<'a> Drop for PortRenderFrame<'a> {
    fn drop(&mut self) {
        self.queue.barrier_sync(|| {});
    }
}

impl<'a> PortRenderFrame<'a> {
    /// Construct a `PortRenderFrame`. Start rendering work.
    pub fn new(
        queue: &xdispatch::Queue,
        frame: &'a PresenterFrame,
        root: &Option<NodeRef>,
        gfx_objects: &GfxObjects,
        temp_res_pool: &mut TempResPool,
        mut temp_res_table: &mut TempResTable,
        image_memory_type: gfx::MemoryType,
        port_manager: &mut PortManager,
    ) -> Result<Self> {
        fn traverse<F: FnMut(&Layer) -> Result<()>>(
            layer: &Layer,
            frame: &PresenterFrame,
            f: &mut F,
        ) -> Result<()> {
            if let &Some(ref child) = layer.child.read_presenter(frame).unwrap() {
                child.for_each_node_of_r(|layer: &Layer| traverse(layer, frame, f))?;
            }

            if let &Some(ref mask) = layer.mask.read_presenter(frame).unwrap() {
                mask.for_each_node_of_r(|layer: &Layer| traverse(layer, frame, f))?;
            }

            f(layer)
        }

        // Block until the previous encoding task is done. (We don't want
        // `Mutex::lock()` inside a dispatch task to block)
        queue.barrier_sync(|| {});

        // Scan the uses of `Port`s
        let mut outputs = HashMap::new();

        if let &Some(ref root) = root {
            root.for_each_node_of_r(|layer: &Layer| {
                traverse(layer, frame, &mut |layer| {
                    let contents = layer.contents.read_presenter(frame).unwrap();

                    if let &LayerContents::Port(ref port) = contents {
                        let port_instance = port_manager.get(port, gfx_objects).clone();

                        let image;
                        let image_extents;
                        let image_format;
                        let fence;
                        {
                            let port_instance = port_instance.lock().unwrap();

                            // Create a backing store image
                            let ref device = gfx_objects.device;
                            image_format = port_instance.image_format();
                            image_extents = port_instance.image_extents();
                            let image_usage =
                                port_instance.image_usage() | gfx::ImageUsageFlags::Sampled;
                            image = device
                                .build_image()
                                .extents(&image_extents)
                                .format(image_format)
                                .usage(image_usage)
                                .build()?;
                            temp_res_pool.bind(&mut temp_res_table, image_memory_type, &image)?;

                            // Create a fence
                            fence = gfx_objects.main_queue.queue.new_fence()?;

                            outputs.insert(
                                port.clone(),
                                PortRenderOutput {
                                    image: image.clone(),
                                    fence: fence.clone(),
                                },
                            );
                        }

                        // Provoke rendering work
                        let mut render_context = PortRenderContext {
                            image,
                            image_props: PortImageProps {
                                extents: image_extents,
                                format: image_format,
                            },
                            fence,
                            schedule_next_frame: false,
                        };

                        // Extend the lifetime. Should be okay since we insert
                        // `barrier_sync` at `Self::drop` which blocks until all
                        // dispatches are done
                        let frame = unsafe { &*(frame as *const PresenterFrame) };

                        queue.async(move || {
                            port_instance
                                .lock()
                                .unwrap()
                                .render(&mut render_context, frame)
                                .unwrap();
                        });
                    }

                    Ok(())
                })
            })?;
        }

        Ok(Self {
            queue: queue.clone(),
            outputs,
            frame,
        })
    }

    pub fn get_output(&'a self, port: &RefEqArc<Port>) -> Option<&'a PortRenderOutput> {
        self.outputs.get(port)
    }
}
