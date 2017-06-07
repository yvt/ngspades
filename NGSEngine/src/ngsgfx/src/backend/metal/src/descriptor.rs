
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;
use atomic_refcell::{AtomicRefCell, AtomicRef};

use {OCPtr, RefEqArc};
use imp::{Backend, Buffer, ImageView, Sampler};

const NUM_STAGES: usize = 4;
const VERTEX_STAGE_INDEX: usize = 0;
const FRAGMENT_STAGE_INDEX: usize = 1;
const COMPUTE_STAGE_INDEX: usize = 2;
const COPY_STAGE_INDEX: usize = 3; // for descriptor copy

/// Fake descriptor pool implementation.
///
/// Always allocates from a global heap.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DescriptorPool {
    data: RefEqArc<DescriptorPoolData>,
}

#[derive(Debug)]
struct DescriptorPoolData {}

impl core::DescriptorPool<Backend> for DescriptorPool {
    type Allocation = ();

    fn deallocate(&mut self, allocation: &mut Self::Allocation) {
    }

    fn make_descriptor_set(&mut self,
                           description: &core::DescriptorSetDescription<DescriptorSetLayout>)
                           -> core::Result<Option<(DescriptorSet, Self::Allocation)>> {
        Ok(Some((DescriptorSet::new(description.layout)?, ())))
    }

    fn reset(&mut self) {
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DescriptorSet {
    data: RefEqArc<DescriptorSetData>,
}

struct DescriptorSetData {
    layout: DescriptorSetLayout,
    table: AtomicRefCell<DescriptorSetTable>,
}

// why do people keep forgetting to implement Debug
impl ::std::fmt::Debug for DescriptorSetData {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        fmt.debug_struct("DescriptorSetData")
            .field("table", &*self.table.borrow())
            .finish()
    }
}

#[derive(Debug, Default)]
struct DescriptorSetTable {
    stages: [DescriptorSetTableStage; NUM_STAGES],
}

#[derive(Debug, Default)]
struct DescriptorSetTableStage {
    image_views: Vec<Option<ImageView>>,
    buffers: Vec<Option<(Buffer, usize)>>,
    samplers: Vec<Option<Sampler>>,
}

type DescriptorTuple<'a> = (Option<&'a core::DescriptorImage<'a, Backend>>,
                            Option<&'a core::DescriptorBuffer<'a, Backend>>,
                            Option<&'a Sampler>);

impl core::DescriptorSet<Backend> for DescriptorSet {
    fn update(&self, writes: &[core::WriteDescriptorSet<Backend>]) {
        let mut table = self.data.table.borrow_mut();

        for write in writes {
            match write.elements {
                core::WriteDescriptors::StorageImage(e) => {
                    self.update_inner(&mut *table, write, e.iter().map(|x| (Some(x), None, None)));
                }
                core::WriteDescriptors::SampledImage(e) => {
                    self.update_inner(&mut *table, write, e.iter().map(|x| (Some(x), None, None)));
                }
                core::WriteDescriptors::Sampler(e) => {
                    self.update_inner(&mut *table, write, e.iter().map(|x| (None, None, Some(*x))));
                }
                core::WriteDescriptors::CombinedImageSampler(e) => {
                    self.update_inner(&mut *table,
                                      write,
                                      e.iter().map(|x| (Some(&x.0), None, Some(x.1))));
                }
                core::WriteDescriptors::ConstantBuffer(e) => {
                    self.update_inner(&mut *table, write, e.iter().map(|x| (None, Some(x), None)));
                }
                core::WriteDescriptors::StorageBuffer(e) => {
                    self.update_inner(&mut *table, write, e.iter().map(|x| (None, Some(x), None)));
                }
                core::WriteDescriptors::DynamicConstantBuffer(e) => {
                    self.update_inner(&mut *table, write, e.iter().map(|x| (None, Some(x), None)));
                }
                core::WriteDescriptors::DynamicStorageBuffer(e) => {
                    self.update_inner(&mut *table, write, e.iter().map(|x| (None, Some(x), None)));
                }
                core::WriteDescriptors::InputAttachment(e) => {
                    self.update_inner(&mut *table, write, e.iter().map(|x| (Some(x), None, None)));
                }
            }
        }
    }

    fn copy_from(&self, copies: &[core::CopyDescriptorSet<Self>]) {
        let mut dest_table = self.data.table.borrow_mut();
        let ref dest_layout = self.data.layout.data;

        for copy in copies {
            // prevent double borrow (which would result in a panic)
            let src_table = if copy.source == self {
                None
            } else {
                Some(copy.source.data.table.borrow())
            };

            let ref src_layout = copy.source.data.layout.data;

            let mut dest_binding_loc = copy.destination_binding;
            let mut dest_index = copy.destination_index;
            let mut src_binding_loc = copy.source_binding;
            let mut src_index = copy.source_index;
            let mut num_elements = copy.num_elements;

            while num_elements > 0 {
                assert!(dest_binding_loc < dest_layout.bindings.len(),
                        "out of range: descriptor count or destination start binding location");
                let dest_binding: &DescriptorSetLayoutBinding =
                    dest_layout.bindings[src_binding_loc]
                        .as_ref()
                        .expect("no binding at the destination location");
                assert!(dest_index < dest_binding.num_elements,
                        "out of range: destination start index");

                assert!(src_binding_loc < src_layout.bindings.len(),
                        "out of range: descriptor count or source start binding location");
                let src_binding: &DescriptorSetLayoutBinding =
                    src_layout.bindings[src_binding_loc]
                        .as_ref()
                        .expect("no binding at the source location");
                assert!(src_index < src_binding.num_elements,
                        "out of range: source start index");

                let copy_count = *[num_elements,
                                   src_binding.num_elements - src_index,
                                   dest_binding.num_elements - dest_index]
                                          .iter()
                                          .min()
                                          .unwrap();

                assert!(copy_count > 0);
                assert_eq!(dest_binding.descriptor_type, src_binding.descriptor_type);


                for i in 0..NUM_STAGES {
                    if let Some(src_table_index) = src_binding.image_view_index[COPY_STAGE_INDEX] {
                        if let Some(dest_table_index) = dest_binding.image_view_index[i] {
                            if let Some(ref src_table) = src_table {
                                let ref mut dest_stage_table = dest_table.stages[i];
                                let ref src_copy_table = src_table.stages[COPY_STAGE_INDEX];
                                dest_stage_table.image_views[dest_table_index + dest_index ..
                                    dest_table_index + dest_index + copy_count]
                                    .clone_from_slice(
                                        &src_copy_table.image_views[src_table_index + src_index ..
                                            src_table_index + src_index + copy_count]
                                    );
                            } else {
                                for k in 0..copy_count {
                                    dest_table.stages[i].image_views[dest_table_index + dest_index + k]
                                        = dest_table.stages[COPY_STAGE_INDEX].image_views[src_table_index + src_index + k].clone();
                                }
                            }
                        }
                    }

                    if let Some(src_table_index) = src_binding.buffer_index[COPY_STAGE_INDEX] {
                        if let Some(dest_table_index) = dest_binding.buffer_index[i] {
                            if let Some(ref src_table) = src_table {
                                let ref mut dest_stage_table = dest_table.stages[i];
                                let ref src_copy_table = src_table.stages[COPY_STAGE_INDEX];
                                dest_stage_table.buffers[dest_table_index + dest_index ..
                                    dest_table_index + dest_index + copy_count]
                                    .clone_from_slice(
                                        &src_copy_table.buffers[src_table_index + src_index ..
                                            src_table_index + src_index + copy_count]
                                    );
                            } else {
                                for k in 0..copy_count {
                                    dest_table.stages[i].buffers[dest_table_index + dest_index + k]
                                        = dest_table.stages[COPY_STAGE_INDEX].buffers[src_table_index + src_index + k].clone();
                                }
                            }
                        }
                    }

                    if let Some(src_table_index) = src_binding.sampler_index[COPY_STAGE_INDEX] {
                        if let Some(dest_table_index) = dest_binding.sampler_index[i] {
                            if let Some(ref src_table) = src_table {
                                let ref mut dest_stage_table = dest_table.stages[i];
                                let ref src_copy_table = src_table.stages[COPY_STAGE_INDEX];
                                dest_stage_table.samplers[dest_table_index + dest_index ..
                                    dest_table_index + dest_index + copy_count]
                                    .clone_from_slice(
                                        &src_copy_table.samplers[src_table_index + src_index ..
                                            src_table_index + src_index + copy_count]
                                    );
                            } else {
                                for k in 0..copy_count {
                                    dest_table.stages[i].samplers[dest_table_index + dest_index + k]
                                        = dest_table.stages[COPY_STAGE_INDEX].samplers[src_table_index + src_index + k].clone();
                                }
                            }
                        }
                    }
                }

                src_index += copy_count;
                if src_index == src_binding.num_elements {
                    src_index = 0;
                    src_binding_loc += 1;
                }
                while src_binding_loc < src_layout.bindings.len() &&
                      src_layout.bindings[src_binding_loc].is_none() {
                    src_binding_loc += 1;
                }

                dest_index += copy_count;
                if dest_index == dest_binding.num_elements {
                    dest_index = 0;
                    dest_binding_loc += 1;
                }
                while dest_binding_loc < dest_layout.bindings.len() &&
                      dest_layout.bindings[dest_binding_loc].is_none() {
                    dest_binding_loc += 1;
                }

                num_elements -= copy_count;
            }
        }
    }
}

impl DescriptorSet {
    pub(crate) fn new(layout: &DescriptorSetLayout) -> core::Result<Self> {
        let data = DescriptorSetData {
            layout: layout.clone(),
            table: AtomicRefCell::default(),
        };

        {
            let mut table = data.table.borrow_mut();
            for i in 0..NUM_STAGES {
                table.stages[i].image_views = vec![None; layout.data.num_image_views[i]];
                table.stages[i].buffers = vec![None; layout.data.num_buffers[i]];
                table.stages[i].samplers = layout.data.samplers[i].clone(); // immutable samplers
            }
        }

        Ok(DescriptorSet { data: RefEqArc::new(data) })
    }

    fn update_inner<'a, T>(&self,
                           table: &mut DescriptorSetTable,
                           wds: &core::WriteDescriptorSet<Backend>,
                           descs: T)
        where T: Iterator<Item = DescriptorTuple<'a>> + ExactSizeIterator
    {
        let mut binding_loc = wds.start_binding;
        let mut index = wds.start_index;
        let ref layout = *self.data.layout.data;

        // TODO: make use of ExactSizeIterator to accelerate consecutive updates?

        for (image, buffer, sampler) in descs {
            assert!(binding_loc < layout.bindings.len(),
                    "out of range: descriptor count or start binding location");
            let binding: &DescriptorSetLayoutBinding = layout.bindings[binding_loc]
                .as_ref()
                .expect("no binding at the location");
            assert!(index < binding.num_elements, "out of range: start index");

            assert_eq!(binding.descriptor_type, wds.elements.descriptor_type());

            for i in 0..NUM_STAGES {
                if let (Some(image), Some(image_index)) = (image, binding.image_view_index[i]) {
                    table.stages[i].image_views[image_index + index] =
                        Some(image.image_view.clone());
                }
                if let (Some(buffer), Some(buffer_index)) = (buffer, binding.buffer_index[i]) {
                    table.stages[i].buffers[buffer_index + index] = Some((buffer.buffer.clone(),
                                                                          buffer.offset));
                }
                if let (Some(sampler), Some(sampler_index)) = (sampler, binding.sampler_index[i]) {
                    table.stages[i].samplers[sampler_index + index] = Some(sampler.clone());
                }
            }

            index += 1;
            if index == binding.num_elements {
                index = 0;
                binding_loc += 1;
            }
            while binding_loc < layout.bindings.len() && layout.bindings[binding_loc].is_none() {
                binding_loc += 1;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DescriptorSetLayout {
    data: RefEqArc<DescriptorSetLayoutData>,
}

#[derive(Debug)]
struct DescriptorSetLayoutData {
    num_image_views: [usize; NUM_STAGES],
    num_buffers: [usize; NUM_STAGES],
    /// Used to preinitialize a descriptor set with static samplers.
    samplers: [Vec<Option<Sampler>>; NUM_STAGES],
    bindings: Vec<Option<DescriptorSetLayoutBinding>>,
}

#[derive(Debug, Clone)]
struct DescriptorSetLayoutBinding {
    descriptor_type: core::DescriptorType,
    num_elements: usize,
    image_view_index: [Option<usize>; NUM_STAGES],
    buffer_index: [Option<usize>; NUM_STAGES],
    sampler_index: [Option<usize>; NUM_STAGES],
}

impl core::DescriptorSetLayout for DescriptorSetLayout {}

impl DescriptorSetLayout {
    pub(crate) fn new(desc: &core::DescriptorSetLayoutDescription<Sampler>) -> core::Result<Self> {
        let mut bindings = Vec::new();
        let mut num_image_views = [0; NUM_STAGES];
        let mut num_buffers = [0; NUM_STAGES];
        let mut num_samplers = [0; NUM_STAGES];

        for binding_desc in desc.bindings.iter() {
            let loc = binding_desc.location;
            if loc + 1 > bindings.len() {
                bindings.resize(loc + 1, None);
            }

            let stage_flags = binding_desc.stage_flags;
            let descriptor_type: core::DescriptorType = binding_desc.descriptor_type;
            let has_stage: [bool; NUM_STAGES] =
                [!(stage_flags & core::ShaderStageFlags::Vertex).is_empty(),
                 !(stage_flags & core::ShaderStageFlags::Fragment).is_empty(),
                 !(stage_flags & core::ShaderStageFlags::Compute).is_empty(),
                 true];

            let mut image_view_index = [None; NUM_STAGES];
            let mut buffer_index = [None; NUM_STAGES];
            let mut sampler_index = [None; NUM_STAGES];

            for i in 0..NUM_STAGES {
                if descriptor_type.has_image_view() {
                    image_view_index[i] = Some(num_image_views[i]);
                    num_image_views[i] += binding_desc.num_elements;
                }
                if descriptor_type.has_buffer() {
                    buffer_index[i] = Some(num_buffers[i]);
                    num_buffers[i] += binding_desc.num_elements;
                }
                if descriptor_type.has_sampler() {
                    sampler_index[i] = Some(num_samplers[i]);
                    num_samplers[i] += binding_desc.num_elements;
                }
            }

            let binding = DescriptorSetLayoutBinding {
                descriptor_type,
                num_elements: binding_desc.num_elements,
                image_view_index,
                buffer_index,
                sampler_index,
            };

            assert!(bindings[loc].is_none(),
                    "duplicate binding location: {}",
                    loc);
            bindings[loc] = Some(binding);
        }

        // Preinitialize immutable samplers
        let mut samplers: [Vec<Option<Sampler>>; NUM_STAGES] = Default::default();

        for i in 0..NUM_STAGES {
            samplers[i] = vec![None; num_samplers[i]];
        }

        for binding_desc in desc.bindings.iter() {
            if let Some(imut_samplers) = binding_desc.immutable_samplers {
                let binding = bindings[binding_desc.location].as_ref().unwrap();
                assert_eq!(imut_samplers.len(), binding_desc.num_elements);

                for i in 0..NUM_STAGES {
                    if let Some(start_index) = binding.sampler_index[i] {
                        for k in 0..binding_desc.num_elements {
                            samplers[i][start_index + k] = Some(imut_samplers[k].clone());
                        }
                    }
                }
            }
        }

        let data = DescriptorSetLayoutData {
            num_image_views,
            num_buffers,
            samplers,
            bindings,
        };
        Ok(DescriptorSetLayout { data: RefEqArc::new(data) })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineLayout {
    data: RefEqArc<PipelineLayoutData>,
}

#[derive(Debug)]
struct PipelineLayoutData {
    descriptor_sets: Vec<PipelineLayoutDescriptorSet>,
    num_image_views: [usize; NUM_STAGES],
    num_buffers: [usize; NUM_STAGES],
    num_samplers: [usize; NUM_STAGES],
}

#[derive(Debug)]
struct PipelineLayoutDescriptorSet {
    layout: DescriptorSetLayout,
    image_view_index: [usize; NUM_STAGES],
    buffer_index: [usize; NUM_STAGES],
    sampler_index: [usize; NUM_STAGES],
}

impl core::PipelineLayout for PipelineLayout {}

impl PipelineLayout {
    pub(crate) fn new(desc: &core::PipelineLayoutDescription<DescriptorSetLayout>)
                      -> core::Result<Self> {
        let descriptor_set_layouts: &[&DescriptorSetLayout] = desc.descriptor_set_layouts;
        let mut num_image_views = [0; NUM_STAGES];
        let mut num_buffers = [0; NUM_STAGES];
        let mut num_samplers = [0; NUM_STAGES];

        let descriptor_sets: Vec<PipelineLayoutDescriptorSet> = descriptor_set_layouts
            .iter()
            .map(|&ds_layout| {
                let ds = PipelineLayoutDescriptorSet {
                    layout: ds_layout.clone(),
                    image_view_index: num_image_views.clone(),
                    buffer_index: num_buffers.clone(),
                    sampler_index: num_samplers.clone(),
                };
                for i in 0..NUM_STAGES {
                    num_image_views[i] += ds.layout.data.num_image_views[i];
                    num_buffers[i] += ds.layout.data.num_buffers[i];
                    num_samplers[i] += ds.layout.data.samplers[i].len();
                }
                ds
            })
            .collect();

        // TODO: check hardware capabilities?

        let data = PipelineLayoutData {
            descriptor_sets,
            num_image_views,
            num_buffers,
            num_samplers,
        };
        Ok(PipelineLayout { data: RefEqArc::new(data) })
    }
}
