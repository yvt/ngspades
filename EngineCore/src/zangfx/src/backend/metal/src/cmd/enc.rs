//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use zangfx_metal_rs as metal;
use zangfx_base::{self as base, command, heap};
use arrayvec::ArrayVec;
use std::collections::HashSet;

use crate::cmd::fence::Fence;
use crate::buffer::Buffer;
use crate::image::Image;
use crate::heap::{BufferHeap, EmulatedHeap, Heap};

#[derive(Debug, Default)]
crate struct CmdBufferFenceSet {
    crate wait_fences: Vec<Fence>,
    crate signal_fences: HashSet<Fence>,
}

impl CmdBufferFenceSet {
    crate fn new() -> Self {
        Default::default()
    }

    crate fn wait_fence(&mut self, fence: Fence) {
        if self.signal_fences.contains(&fence) {
            // Found a matching fence signaling operating in the same CB
            return;
        }
        self.wait_fences.push(fence);
    }

    crate fn signal_fence(&mut self, fence: Fence) {
        self.signal_fences.insert(fence);
    }
}

fn translate_resource(handle: base::ResourceRef) -> (metal::MTLResource, bool) {
    match handle {
        base::ResourceRef::Buffer(buffer) => {
            let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
            (
                *my_buffer.metal_buffer_and_offset().unwrap().0,
                my_buffer.is_subbuffer(),
            )
        }
        base::ResourceRef::Image(image) => {
            let my_image: &Image = image.downcast_ref().expect("bad image type");
            (*my_image.metal_texture(), false)
        }
    }
}

crate trait UseResources {
    fn use_metal_resources(&self, resources: &[metal::MTLResource], usage: metal::MTLResourceUsage);
    fn use_metal_heaps(&self, heaps: &[metal::MTLHeap]);

    fn use_gfx_resource(&self, usage: command::ResourceUsageFlags, objs: base::ResourceSet) {
        let metal_usage = unimplemented!();/* match usage {
            command::ResourceUsage::Read => metal::MTLResourceUsage::Read,
            command::ResourceUsage::Write => metal::MTLResourceUsage::Write,
            command::ResourceUsage::Sample => metal::MTLResourceUsage::Sample,
        }; */

        unimplemented!()
        /* for objs in objs.chunks(256) {
            let mut metal_usage = metal_usage;
            let metal_resources: ArrayVec<[_; 256]> = objs.iter()
                .cloned()
                .map(|r| {
                    let (metal_resource, is_subbuffer) = translate_resource(r);
                    if is_subbuffer {
                        // This resource is a suballocated portion of
                        // `BufferHeap`, a `MTLBuffer`-backed heap. The
                        // application might call `use_resource` on multiple
                        // resources from the heap with different resource usage
                        // type.
                        //
                        // In such situations, the usage type is overwritten
                        // every time `use_metal_resources` is called. To be
                        // safe, use the `Write` usage type for such resources.
                        metal_usage = metal::MTLResourceUsage::Write;
                    }
                    metal_resource
                })
                .collect();
            self.use_metal_resources(metal_resources.as_slice(), metal_usage);
        } */
    }

    fn use_gfx_heap(&self, heaps: &[&heap::HeapRef]) {
        use zangfx_metal_rs::MTLResourceUsage::Read;
        let mut metal_heaps = ArrayVec::<[_; 256]>::new();
        let mut metal_resources = ArrayVec::<[_; 256]>::new();

        for heap in heaps {
            if let Some(heap) = heap.query_ref::<Heap>() {
                metal_heaps.push(heap.metal_heap());
                if metal_heaps.len() == metal_heaps.capacity() {
                    self.use_metal_heaps(metal_heaps.as_slice());
                    metal_heaps.clear();
                }
            } else if let Some(heap) = heap.query_ref::<BufferHeap>() {
                metal_resources.push(*heap.metal_buffer());
                if metal_resources.len() == metal_resources.capacity() {
                    self.use_metal_resources(metal_resources.as_slice(), Read);
                    metal_resources.clear();
                }
            } else if let Some(heap) = heap.query_ref::<EmulatedHeap>() {
                heap.for_each_metal_resources(&mut |metal_resource| {
                    metal_resources.push(metal_resource);
                    if metal_resources.len() == metal_resources.capacity() {
                        self.use_metal_resources(metal_resources.as_slice(), Read);
                        metal_resources.clear();
                    }
                });
            } else {
                panic!("invalid heap type");
            }
        }

        if metal_heaps.len() > 0 {
            self.use_metal_heaps(metal_heaps.as_slice());
        }
        if metal_resources.len() > 0 {
            self.use_metal_resources(metal_resources.as_slice(), Read);
        }
    }
}

impl UseResources for metal::MTLRenderCommandEncoder {
    fn use_metal_resources(
        &self,
        resources: &[metal::MTLResource],
        usage: metal::MTLResourceUsage,
    ) {
        self.use_resources(resources, usage)
    }

    fn use_metal_heaps(&self, heaps: &[metal::MTLHeap]) {
        self.use_heaps(heaps)
    }
}

impl UseResources for metal::MTLComputeCommandEncoder {
    fn use_metal_resources(
        &self,
        resources: &[metal::MTLResource],
        usage: metal::MTLResourceUsage,
    ) {
        self.use_resources(resources, usage)
    }

    fn use_metal_heaps(&self, heaps: &[metal::MTLHeap]) {
        self.use_heaps(heaps)
    }
}

crate trait DebugCommands {
    fn begin_debug_group(&self, label: &str);
    fn end_debug_group(&self);
    fn debug_marker(&self, label: &str);
}

impl DebugCommands for metal::MTLCommandEncoder {
    fn begin_debug_group(&self, label: &str) {
        self.push_debug_group(label);
    }

    fn end_debug_group(&self) {
        self.pop_debug_group();
    }

    fn debug_marker(&self, label: &str) {
        self.insert_debug_signpost(label);
    }
}
