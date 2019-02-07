//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use refeq::RefEqArc;
use std::{collections::HashMap, marker::PhantomData};
use xdispatch;
use zangfx::{base as gfx, base::Result};

use ngspf_core::{NodeRef, PresenterFrame};

use crate::layer::{Layer, LayerContents};
use crate::port::{GfxObjects, Port, PortFrame, PortImageProps, PortInstance, PortRenderContext};
use crate::temprespool::{TempResPool, TempResTable};

/// Maintains port instances associated with `Port`s and persists them between
/// frames.
#[derive(Debug)]
pub(super) struct PortManager {
    /// Set of mounted port instances.
    port_map: HashMap<RefEqArc<Port>, PortMapping>,
}

#[derive(Debug)]
struct PortMapping {
    instance: Box<dyn PortInstance>,
    used_in_last_frame: bool,
}

impl PortManager {
    pub fn new() -> Self {
        Self {
            port_map: HashMap::new(),
        }
    }

    fn instantiate_if_not_exist(&mut self, port: &RefEqArc<Port>, gfx_objects: &GfxObjects) {
        let ent = self.port_map.entry(RefEqArc::clone(port));
        let map = ent.or_insert_with(|| {
            // The port instance has not yet been created for the `Port`.
            // Mount the port and create the port instance.
            let instance = port.mount(gfx_objects);

            // Save the created instance and return a reference to it
            PortMapping {
                instance,
                used_in_last_frame: true,
            }
        });
        map.used_in_last_frame = true;
    }
}

/// Manages rendering work on `Port`s in a certain frame.
#[derive(Debug)]
pub(crate) struct PortRenderFrame<'a> {
    outputs: HashMap<RefEqArc<Port>, PortRenderOutput>,
    queue: xdispatch::Queue,

    /// `PresenterFrame` must outlive `self`. Otherwise `PortInstance::render`
    /// would access the destroyed `PresenterFrame`.
    _frame: PhantomData<&'a PresenterFrame>,

    /// `PortManager` must outlive `self`. Otherwise `PortInstance::render`
    /// would access the destroyed `PortManager`.
    _port_manager: PhantomData<&'a mut PortManager>,
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
    ///
    /// # Safety
    ///
    /// The constructed `PortRenderFrame` must not be disposed without dropping
    /// (e.g., passed to `std::mem::forget`).
    pub unsafe fn new(
        queue: &xdispatch::Queue,
        frame: &'a PresenterFrame,
        root: &Option<NodeRef>,
        gfx_objects: &GfxObjects,
        temp_res_pool: &mut TempResPool,
        mut temp_res_table: &mut TempResTable,
        image_memory_type: gfx::MemoryType,
        port_manager: &'a mut PortManager,
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

        for map in port_manager.port_map.values_mut() {
            map.used_in_last_frame = false;
        }

        // Scan the uses of `Port`s
        let mut outputs = HashMap::new();

        if let &Some(ref root) = root {
            root.for_each_node_of_r(|layer: &Layer| {
                traverse(layer, frame, &mut |layer| {
                    let contents = layer.contents.read_presenter(frame).unwrap();

                    if let &LayerContents::Port(ref port) = contents {
                        port_manager.instantiate_if_not_exist(port, gfx_objects);
                    }

                    Ok(())
                })
            })?;
        }

        // Dipose out-dated instances
        port_manager
            .port_map
            .retain(|_, map| map.used_in_last_frame);

        // Extend the lifetime. Should be okay since we insert
        // `barrier_sync` at `Self::drop` which blocks until all
        // dispatches are done
        let frame = &*(frame as *const PresenterFrame);

        for (port, map) in port_manager.port_map.iter_mut() {
            let mut port_frame: Box<dyn PortFrame> = map.instance.start_frame(frame)?;

            // Create a backing store image
            let ref device = gfx_objects.device;
            let image_format = port_frame.image_format();
            let image_extents = port_frame.image_extents();
            let image_usage = port_frame.image_usage() | gfx::ImageUsageFlags::SAMPLED;
            let image = device
                .build_image()
                .extents(&image_extents)
                .format(image_format)
                .usage(image_usage)
                .build()?;
            temp_res_pool.bind(&mut temp_res_table, image_memory_type, &image)?;

            // Create a fence
            let fence = gfx_objects.main_queue.queue.new_fence()?;

            outputs.insert(
                port.clone(),
                PortRenderOutput {
                    image: image.clone(),
                    fence: fence.clone(),
                },
            );

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

            // Extend the lifetime of inner references. Should be okay
            // since we have a mutable reference to `port_manager` and we
            // don't intend to access `map.instance` via other means until
            // the next `barrier_sync`.
            use std::mem::transmute;
            let mut port_frame: Box<dyn PortFrame + 'static> = transmute(port_frame);

            queue.r#async(move || {
                // TODO: Handle GFX errors
                port_frame.render(&mut render_context).unwrap();
            });
        }

        Ok(Self {
            queue: queue.clone(),
            outputs,
            _frame: PhantomData,
            _port_manager: PhantomData,
        })
    }

    pub fn get_output(&'a self, port: &RefEqArc<Port>) -> Option<&'a PortRenderOutput> {
        self.outputs.get(port)
    }
}
