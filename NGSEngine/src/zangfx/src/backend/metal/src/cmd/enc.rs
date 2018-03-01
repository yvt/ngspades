//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use metal;
use base::{command, handles, heap};
use smallvec::SmallVec;

use cmd::fence::Fence;
use buffer::Buffer;
use heap::{EmulatedHeap, Heap};

#[derive(Debug, Default)]
pub struct CmdBufferFenceSet {
    pub wait_fences: Vec<Fence>,
    pub signal_fences: Vec<Fence>,
}

impl CmdBufferFenceSet {
    pub fn new() -> Self {
        Default::default()
    }
}

fn translate_resource(handle: handles::ResourceRef) -> metal::MTLResource {
    match handle {
        handles::ResourceRef::Buffer(buffer) => {
            let my_buffer: &Buffer = buffer.downcast_ref().expect("bad buffer type");
            *my_buffer.metal_buffer()
        }
        handles::ResourceRef::Image(_image) => unimplemented!(),
    }
}

pub trait UseResources {
    fn use_metal_resources(&self, resources: &[metal::MTLResource], usage: metal::MTLResourceUsage);
    fn use_metal_heaps(&self, heaps: &[metal::MTLHeap]);

    fn use_gfx_resource(&self, usage: command::ResourceUsage, objs: &[handles::ResourceRef]) {
        let metal_usage = match usage {
            command::ResourceUsage::Read => metal::MTLResourceUsage::Read,
            command::ResourceUsage::Write => metal::MTLResourceUsage::Write,
            command::ResourceUsage::Sample => metal::MTLResourceUsage::Sample,
        };

        for objs in objs.chunks(256) {
            let metal_resources: SmallVec<[_; 256]> =
                objs.iter().cloned().map(translate_resource).collect();
            self.use_metal_resources(metal_resources.as_slice(), metal_usage);
        }
    }

    fn use_gfx_heap(&self, heaps: &[&heap::Heap]) {
        use metal::MTLResourceUsage::Read;
        let mut metal_heaps = SmallVec::<[_; 256]>::new();
        let mut metal_resources = SmallVec::<[_; 256]>::new();

        for heap in heaps {
            if let Some(heap) = heap.query_ref::<Heap>() {
                metal_heaps.push(heap.metal_heap());
                if metal_heaps.len() == metal_heaps.capacity() {
                    self.use_metal_heaps(metal_heaps.as_slice());
                    metal_heaps.clear();
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
