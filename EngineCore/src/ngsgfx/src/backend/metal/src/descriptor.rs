//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use metal;
use atomic_refcell::AtomicRefCell;
use spirv_cross::{SpirV2Msl, ExecutionModel, ResourceBinding};

use std::sync::{Mutex, Arc};

use {RefEqBox, RefEqArc};
use imp::{Backend, Buffer, ImageView, Sampler, DeviceData};

const NUM_STAGES: usize = 4;
const NUM_REAL_STAGES: usize = 3;
const VERTEX_STAGE_INDEX: usize = 0;
const FRAGMENT_STAGE_INDEX: usize = 1;
const COMPUTE_STAGE_INDEX: usize = 2;
/// The stage index for an imaginary argument table, used during copies between
/// descriptor sets. (This is required because descriptors are not required to
/// be required by any shader stages)
const COPY_STAGE_INDEX: usize = 3;

/// Fake descriptor pool implementation.
///
/// Always allocates from a global heap.
#[derive(Debug)]
pub struct DescriptorPool {
    data: RefEqBox<DescriptorPoolData>,
}

#[derive(Debug)]
struct DescriptorPoolData {}

impl core::DescriptorPool<Backend> for DescriptorPool {
    type Allocation = ();

    fn deallocate(&mut self, _: &mut Self::Allocation) {}

    fn make_descriptor_set(
        &mut self,
        description: &core::DescriptorSetDescription<DescriptorSetLayout>,
    ) -> core::Result<Option<(DescriptorSet, Self::Allocation)>> {
        Ok(Some((DescriptorSet::new(description.layout)?, ())))
    }

    fn reset(&mut self) {}
}

impl DescriptorPool {
    pub(crate) fn new(_: &Arc<DeviceData>, _: &core::DescriptorPoolDescription) -> Self {
        Self { data: RefEqBox::new(DescriptorPoolData {}) }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DescriptorSet {
    data: RefEqArc<DescriptorSetData>,
}

struct DescriptorSetData {
    layout: DescriptorSetLayout,
    table: AtomicRefCell<DescriptorSetTable>,
    label: Mutex<Option<String>>,
}

// why do people keep forgetting to implement Debug
impl ::std::fmt::Debug for DescriptorSetData {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        fmt.debug_struct("DescriptorSetData")
            .field("layout", &self.layout)
            .field("table", &*self.table.borrow())
            .field("label", &self.label)
            .finish()
    }
}

impl core::Marker for DescriptorSet {
    fn set_label(&self, label: Option<&str>) {
        *self.data.label.lock().unwrap() = label.map(String::from);
    }
}

#[derive(Debug, Default)]
struct DescriptorSetTable {
    stages: [DescriptorSetTableStage; NUM_STAGES],
}

#[derive(Debug, Default)]
struct DescriptorSetTableStage {
    image_views: Vec<Option<ImageView>>,
    buffers: Vec<Option<(Buffer, core::DeviceSize)>>,
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
                    self.update_inner(
                        &mut *table,
                        write,
                        e.iter().map(|x| (Some(&x.0), None, Some(x.1))),
                    );
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
                assert!(
                    dest_binding_loc < dest_layout.bindings.len(),
                    "out of range: descriptor count or destination start binding location"
                );
                let dest_binding: &DescriptorSetLayoutBinding =
                    dest_layout.bindings[src_binding_loc].as_ref().expect(
                        "no binding at the destination location",
                    );
                assert!(
                    dest_index < dest_binding.num_elements,
                    "out of range: destination start index"
                );

                assert!(
                    src_binding_loc < src_layout.bindings.len(),
                    "out of range: descriptor count or source start binding location"
                );
                let src_binding: &DescriptorSetLayoutBinding =
                    src_layout.bindings[src_binding_loc].as_ref().expect(
                        "no binding at the source location",
                    );
                assert!(
                    src_index < src_binding.num_elements,
                    "out of range: source start index"
                );

                let copy_count = *[
                    num_elements,
                    src_binding.num_elements - src_index,
                    dest_binding.num_elements - dest_index,
                ].iter()
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
                                dest_stage_table.image_views[dest_table_index + dest_index..
                                                                 dest_table_index + dest_index +
                                                                     copy_count]
                                    .clone_from_slice(
                                        &src_copy_table.image_views[src_table_index + src_index..
                                                                        src_table_index +
                                                                            src_index +
                                                                            copy_count],
                                    );
                            } else {
                                for k in 0..copy_count {
                                    dest_table.stages[i].image_views[dest_table_index + dest_index +
                                                                         k] =
                                        dest_table.stages[COPY_STAGE_INDEX].image_views
                                            [src_table_index + src_index + k]
                                            .clone();
                                }
                            }
                        }
                    }

                    if let Some(src_table_index) = src_binding.buffer_index[COPY_STAGE_INDEX] {
                        if let Some(dest_table_index) = dest_binding.buffer_index[i] {
                            if let Some(ref src_table) = src_table {
                                let ref mut dest_stage_table = dest_table.stages[i];
                                let ref src_copy_table = src_table.stages[COPY_STAGE_INDEX];
                                dest_stage_table.buffers[dest_table_index + dest_index..
                                                             dest_table_index + dest_index +
                                                                 copy_count]
                                    .clone_from_slice(
                                        &src_copy_table.buffers[src_table_index + src_index..
                                                                    src_table_index + src_index +
                                                                        copy_count],
                                    );
                            } else {
                                for k in 0..copy_count {
                                    dest_table.stages[i].buffers[dest_table_index + dest_index +
                                                                     k] =
                                        dest_table.stages[COPY_STAGE_INDEX].buffers
                                            [src_table_index + src_index + k]
                                            .clone();
                                }
                            }
                        }
                    }

                    if let Some(src_table_index) = src_binding.sampler_index[COPY_STAGE_INDEX] {
                        if let Some(dest_table_index) = dest_binding.sampler_index[i] {
                            if let Some(ref src_table) = src_table {
                                let ref mut dest_stage_table = dest_table.stages[i];
                                let ref src_copy_table = src_table.stages[COPY_STAGE_INDEX];
                                dest_stage_table.samplers[dest_table_index + dest_index..
                                                              dest_table_index + dest_index +
                                                                  copy_count]
                                    .clone_from_slice(
                                        &src_copy_table.samplers[src_table_index + src_index..
                                                                     src_table_index + src_index +
                                                                         copy_count],
                                    );
                            } else {
                                for k in 0..copy_count {
                                    dest_table.stages[i].samplers[dest_table_index + dest_index +
                                                                      k] =
                                        dest_table.stages[COPY_STAGE_INDEX].samplers
                                            [src_table_index + src_index + k]
                                            .clone();
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
                    src_layout.bindings[src_binding_loc].is_none()
                {
                    src_binding_loc += 1;
                }

                dest_index += copy_count;
                if dest_index == dest_binding.num_elements {
                    dest_index = 0;
                    dest_binding_loc += 1;
                }
                while dest_binding_loc < dest_layout.bindings.len() &&
                    dest_layout.bindings[dest_binding_loc].is_none()
                {
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
            label: Mutex::new(None),
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

    fn update_inner<'a, T>(
        &self,
        table: &mut DescriptorSetTable,
        wds: &core::WriteDescriptorSet<Backend>,
        descs: T,
    ) where
        T: Iterator<Item = DescriptorTuple<'a>> + ExactSizeIterator,
    {
        let mut binding_loc = wds.start_binding;
        let mut index = wds.start_index;
        let ref layout = *self.data.layout.data;

        // TODO: make use of ExactSizeIterator to accelerate consecutive updates?

        for (image, buffer, sampler) in descs {
            assert!(
                binding_loc < layout.bindings.len(),
                "out of range: descriptor count or start binding location"
            );
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
                    table.stages[i].buffers[buffer_index + index] =
                        Some((buffer.buffer.clone(), buffer.offset));
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

    pub(crate) fn layout(&self) -> &DescriptorSetLayout {
        &self.data.layout
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
    buffers: [Vec<DescriptorSetBufferLayout>; NUM_REAL_STAGES],
    /// Used to preinitialize a descriptor set with static samplers.
    samplers: [Vec<Option<Sampler>>; NUM_STAGES],
    bindings: Vec<Option<DescriptorSetLayoutBinding>>,

    /// The binding index for each descriptor with dynamic offsets.
    ///
    /// `sum(bindings[x].num_elements for x in dynamic_binding_indexes) == num_dynamic_offsets`
    dynamic_binding_indexes: Vec<usize>,
    num_dynamic_offsets: usize,

    label: Mutex<Option<String>>,
}

/// Describes how each Metal buffer argument table entry is mapped from the NgsGFX representation.
#[derive(Debug, Clone)]
struct DescriptorSetBufferLayout {
    /// Starting dynamic offset index if any.
    dynamic_offset_index: Option<usize>,
}

/// Describes how each descriptor is mapped to Metal argument table entries.
#[derive(Debug, Clone)]
struct DescriptorSetLayoutBinding {
    descriptor_type: core::DescriptorType,
    num_elements: usize,
    image_view_index: [Option<usize>; NUM_STAGES],
    buffer_index: [Option<usize>; NUM_STAGES],
    sampler_index: [Option<usize>; NUM_STAGES],
    dynamic_offset_index: Option<usize>,
}

impl core::DescriptorSetLayout for DescriptorSetLayout {}

impl DescriptorSetLayout {
    pub(crate) fn new(desc: &core::DescriptorSetLayoutDescription<Sampler>) -> core::Result<Self> {
        let mut bindings = Vec::new();
        let mut num_image_views = [0; NUM_STAGES];
        let mut num_buffers = [0; NUM_STAGES];
        let mut buffers: [Vec<DescriptorSetBufferLayout>; NUM_REAL_STAGES] = Default::default();
        let mut num_samplers = [0; NUM_STAGES];

        let mut next_dynamic_binding_index = 0;

        for binding_desc in desc.bindings.iter() {
            let loc = binding_desc.location;
            if loc + 1 > bindings.len() {
                bindings.resize(loc + 1, None);
            }

            let stage_flags = binding_desc.stage_flags;
            let descriptor_type: core::DescriptorType = binding_desc.descriptor_type;
            let has_stage: [bool; NUM_STAGES] =
                [
                    !(stage_flags & core::ShaderStage::Vertex).is_empty(),
                    !(stage_flags & core::ShaderStage::Fragment).is_empty(),
                    !(stage_flags & core::ShaderStage::Compute).is_empty(),
                    true,
                ];

            let dynamic_offset_index;
            if descriptor_type.needs_dynamic_offsets() {
                dynamic_offset_index = Some(next_dynamic_binding_index);
                next_dynamic_binding_index += binding_desc.num_elements;
            } else {
                dynamic_offset_index = None;
            }

            let mut image_view_index = [None; NUM_STAGES];
            let mut buffer_index = [None; NUM_STAGES];
            let mut sampler_index = [None; NUM_STAGES];

            for i in 0..NUM_STAGES {
                if !has_stage[i] {
                    continue;
                }
                if descriptor_type.has_image_view() {
                    image_view_index[i] = Some(num_image_views[i]);
                    num_image_views[i] += binding_desc.num_elements;
                }
                if descriptor_type.has_buffer() {
                    buffer_index[i] = Some(num_buffers[i]);
                    if i < NUM_REAL_STAGES {
                        for k in 0..binding_desc.num_elements {
                            let dyn_off_idx = dynamic_offset_index.map(|x| x + k);
                            buffers[i].push(DescriptorSetBufferLayout {
                                dynamic_offset_index: dyn_off_idx,
                            });
                        }
                    }
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
                dynamic_offset_index,
            };

            assert!(
                bindings[loc].is_none(),
                "duplicate binding location: {}",
                loc
            );
            bindings[loc] = Some(binding);
        }


        // Create a list of bindings with dynamic offsets
        let mut dynamic_binding_indexes = Vec::new();
        for (i, binding) in bindings.iter().enumerate() {
            if let &Some(ref binding) = binding {
                if binding.dynamic_offset_index.is_some() {
                    dynamic_binding_indexes.push(i);
                }
            }
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
            buffers,
            samplers,
            bindings,
            dynamic_binding_indexes,
            num_dynamic_offsets: next_dynamic_binding_index,
            label: Mutex::new(None),
        };
        Ok(DescriptorSetLayout { data: RefEqArc::new(data) })
    }

    pub(crate) fn num_dynamic_offsets(&self) -> usize {
        self.data.num_dynamic_offsets
    }
}

impl core::Marker for DescriptorSetLayout {
    fn set_label(&self, label: Option<&str>) {
        *self.data.label.lock().unwrap() = label.map(String::from);
    }
}

/// Graphics pipeline layout.
///
/// On Metal, each resource type (texture, buffer, and sampler) has its own argument
/// table. Each Vulkan-style descriptor set is mapped to zero or more consecutive
/// elements of argument tables. `PipelineLayoutDescriptorSet` contains the first
/// indices of such elements.
///
/// Additionally, zero or more vertex buffers (defined by `VertexBufferLayoutDescription`)
/// are appended to the end of the vertex shader's buffer argument table.
/// See `GraphicsPipeline` for more about this.
///
/// This is going to change in Metal 2.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineLayout {
    data: RefEqArc<PipelineLayoutData>,
}

#[derive(Debug)]
struct PipelineLayoutData {
    descriptor_sets: Vec<PipelineLayoutDescriptorSet>,
    /// The number of textures in the texture argument table for each stage.
    num_image_views: [usize; NUM_STAGES],
    /// The number of textures in the buffer argument table for each stage.
    ///
    /// Note that vertex shader's buffer argument table is extended by
    /// dedicated vertex buffers (defined by `VertexBufferLayoutDescription`).
    num_buffers: [usize; NUM_STAGES],
    /// The number of textures in the sampler argument table for each stage.
    num_samplers: [usize; NUM_STAGES],

    label: Mutex<Option<String>>,
}

#[derive(Debug)]
struct PipelineLayoutDescriptorSet {
    layout: DescriptorSetLayout,
    /// The first index in the texture argument table for each stage.
    image_view_index: [usize; NUM_STAGES],
    /// The first index in the buffer argument table for each stage.
    buffer_index: [usize; NUM_STAGES],
    /// The first index in the sampler argument table for each stage.
    sampler_index: [usize; NUM_STAGES],
}

impl core::Marker for PipelineLayout {
    fn set_label(&self, label: Option<&str>) {
        *self.data.label.lock().unwrap() = label.map(String::from);
    }
}

impl core::PipelineLayout for PipelineLayout {}

impl PipelineLayout {
    pub(crate) fn new(
        desc: &core::PipelineLayoutDescription<DescriptorSetLayout>,
    ) -> core::Result<Self> {
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
            label: Mutex::new(None),
        };
        Ok(PipelineLayout { data: RefEqArc::new(data) })
    }

    pub(crate) fn setup_spirv2msl(&self, s2m: &mut SpirV2Msl, model: ExecutionModel) {
        for (i, set) in self.data.descriptor_sets.iter().enumerate() {
            set.setup_spirv2msl(s2m, model, i as u32);
        }
    }

    pub(crate) fn num_vertex_shader_buffers(&self) -> usize {
        self.data.num_buffers[VERTEX_STAGE_INDEX]
    }

    pub(crate) fn bind_descriptor_set<T>(
        &self,
        binder: &T,
        set_index: usize,
        set: &DescriptorSet,
        dyn_offs: &[u32],
    ) where
        T: ResourceBinder,
    {
        let ref plds = self.data.descriptor_sets[set_index];
        let set_layout: &DescriptorSetLayout = set.layout();
        let table = set.data.table.borrow(); // DescriptorSetTable

        assert_eq!(
            dyn_offs.len(),
            set_layout.data.num_dynamic_offsets,
            "invalid number of dynamic offsets"
        );

        binder.for_each_stage(|stage| {
            let start_mtl_texture_index = plds.image_view_index[stage];
            let start_mtl_buffer_index = plds.buffer_index[stage];
            let start_mtl_sampler_index = plds.sampler_index[stage];
            let ref table_stage: DescriptorSetTableStage = table.stages[stage];
            let ref buffer_layouts: Vec<DescriptorSetBufferLayout> = set_layout.data.buffers[stage];

            for (i, iv) in table_stage.image_views.iter().enumerate() {
                let texture = iv.as_ref()
                    .expect("found an uninitialized image view descriptor slot")
                    .metal_texture();
                binder.set_texture(stage, start_mtl_texture_index + i, texture);
            }
            for (i, b_off_or_none) in table_stage.buffers.iter().enumerate() {
                let &(ref b, offset) = b_off_or_none.as_ref().expect(
                    "found an uninitialized buffer descriptor slot",
                );
                let buffer = b.metal_buffer();
                let dyn_off_index = buffer_layouts[i].dynamic_offset_index;
                let offset = offset as u32 + dyn_off_index.map(|j| dyn_offs[j]).unwrap_or(0);
                binder.set_buffer(stage, start_mtl_buffer_index + i, buffer, offset);
            }
            for (i, s) in table_stage.samplers.iter().enumerate() {
                let sampler = s.as_ref()
                    .expect("found an uninitialized sampler descriptor slot")
                    .metal_sampler_state();
                binder.set_sampler(stage, start_mtl_sampler_index + i, sampler);
            }
        });
    }

    pub(crate) fn update_dynamic_offset<T>(
        &self,
        binder: &T,
        set_index: usize,
        set: &DescriptorSet,
        dyn_offs: &[u32],
    ) where
        T: ResourceBinder,
    {
        let ref plds = self.data.descriptor_sets[set_index];
        let set_layout: &DescriptorSetLayout = set.layout();
        let table = set.data.table.borrow(); // DescriptorSetTable
        let ref binding_indexes = set_layout.data.dynamic_binding_indexes;
        assert_eq!(
            dyn_offs.len(),
            set_layout.data.num_dynamic_offsets,
            "invalid number of dynamic offsets"
        );
        let mut dyn_off_idx = 0;

        for &binding_index in binding_indexes.iter() {
            let layout_binding = set_layout.data.bindings[binding_index].as_ref().unwrap();

            binder.for_each_stage(|stage| if let Some(buffer_index) =
                layout_binding.buffer_index[stage]
            {
                let start_mtl_buffer_index = plds.buffer_index[stage];
                let ref table_stage: DescriptorSetTableStage = table.stages[stage];
                for k in 0..layout_binding.num_elements {
                    let offset = dyn_offs[dyn_off_idx + k] +
                        table_stage.buffers[buffer_index + k]
                            .as_ref()
                            .expect("found an uninitialized buffer descriptor slot")
                            .1 as u32;
                    let mtl_buffer_index = start_mtl_buffer_index + buffer_index + k;
                    binder.set_buffer_offset(stage, mtl_buffer_index, offset);
                }
            });

            dyn_off_idx += layout_binding.num_elements;
        }
    }
}

pub(crate) trait ResourceBinder {
    fn for_each_stage<T: FnMut(usize)>(&self, cb: T);
    fn set_texture(&self, stage: usize, index: usize, texture: metal::MTLTexture);
    fn set_buffer(&self, stage: usize, index: usize, buffer: metal::MTLBuffer, offset: u32);
    fn set_buffer_offset(&self, stage: usize, index: usize, offset: u32);
    fn set_sampler(&self, stage: usize, index: usize, sampler: metal::MTLSamplerState);
}

pub(crate) struct GraphicsResourceBinder(pub metal::MTLRenderCommandEncoder);
pub(crate) struct ComputeResourceBinder(pub metal::MTLComputeCommandEncoder);

impl ResourceBinder for GraphicsResourceBinder {
    fn for_each_stage<T: FnMut(usize)>(&self, mut cb: T) {
        cb(VERTEX_STAGE_INDEX);
        cb(FRAGMENT_STAGE_INDEX);
    }
    fn set_texture(&self, stage: usize, index: usize, texture: metal::MTLTexture) {
        match stage {
            VERTEX_STAGE_INDEX => self.0.set_vertex_texture(index as u64, texture),
            FRAGMENT_STAGE_INDEX => self.0.set_fragment_texture(index as u64, texture),
            _ => unreachable!(),
        }
    }
    fn set_buffer(&self, stage: usize, index: usize, buffer: metal::MTLBuffer, offset: u32) {
        match stage {
            VERTEX_STAGE_INDEX => {
                self.0.set_vertex_buffer(
                    index as u64,
                    offset as u64,
                    buffer,
                )
            }
            FRAGMENT_STAGE_INDEX => {
                self.0.set_fragment_buffer(
                    index as u64,
                    offset as u64,
                    buffer,
                )
            }
            _ => unreachable!(),
        }
    }
    fn set_buffer_offset(&self, stage: usize, index: usize, offset: u32) {
        match stage {
            VERTEX_STAGE_INDEX => self.0.set_vertex_buffer_offset(index as u64, offset as u64),
            FRAGMENT_STAGE_INDEX => {
                self.0.set_fragment_buffer_offset(
                    index as u64,
                    offset as u64,
                )
            }
            _ => unreachable!(),
        }
    }
    fn set_sampler(&self, stage: usize, index: usize, sampler: metal::MTLSamplerState) {
        match stage {
            VERTEX_STAGE_INDEX => self.0.set_vertex_sampler_state(index as u64, sampler),
            FRAGMENT_STAGE_INDEX => self.0.set_fragment_sampler_state(index as u64, sampler),
            _ => unreachable!(),
        }
    }
}

impl ResourceBinder for ComputeResourceBinder {
    fn for_each_stage<T: FnMut(usize)>(&self, mut cb: T) {
        cb(COMPUTE_STAGE_INDEX);
    }
    fn set_texture(&self, stage: usize, index: usize, texture: metal::MTLTexture) {
        match stage {
            COMPUTE_STAGE_INDEX => self.0.set_texture(index as u64, texture),
            _ => unreachable!(),
        }
    }
    fn set_buffer(&self, stage: usize, index: usize, buffer: metal::MTLBuffer, offset: u32) {
        match stage {
            COMPUTE_STAGE_INDEX => self.0.set_buffer(index as u64, offset as u64, buffer),
            _ => unreachable!(),
        }
    }
    fn set_buffer_offset(&self, stage: usize, index: usize, offset: u32) {
        match stage {
            COMPUTE_STAGE_INDEX => self.0.set_buffer_offset(index as u64, offset as u64),
            _ => unreachable!(),
        }
    }
    fn set_sampler(&self, stage: usize, index: usize, sampler: metal::MTLSamplerState) {
        match stage {
            COMPUTE_STAGE_INDEX => self.0.set_sampler_state(index as u64, sampler),
            _ => unreachable!(),
        }
    }
}

impl PipelineLayoutDescriptorSet {
    fn setup_spirv2msl(&self, s2m: &mut SpirV2Msl, model: ExecutionModel, desc_set_index: u32) {
        let stage_index = match model {
            ExecutionModel::Fragment => FRAGMENT_STAGE_INDEX,
            ExecutionModel::Vertex => VERTEX_STAGE_INDEX,
            ExecutionModel::GLCompute => COMPUTE_STAGE_INDEX,
            _ => unreachable!(),
        };
        let start_image_view_index = self.image_view_index[stage_index];
        let start_buffer_index = self.buffer_index[stage_index];
        let start_sampler_index = self.sampler_index[stage_index];

        for (binding_index, binding /* :&DescriptorSetLayoutBinding */) in
            self.layout.data.bindings.iter().enumerate()
        {
            if let Some(binding) = binding.as_ref() {
                s2m.bind_resource(&ResourceBinding {
                    desc_set: desc_set_index,
                    binding: binding_index as u32,
                    msl_buffer: binding.buffer_index[stage_index].map(|x| {
                        (x + start_buffer_index) as u32
                    }),
                    msl_texture: binding.image_view_index[stage_index].map(|x| {
                        (x + start_image_view_index) as u32
                    }),
                    msl_sampler: binding.sampler_index[stage_index].map(|x| {
                        (x + start_sampler_index) as u32
                    }),
                    stage: model,
                });
            }
        }
    }
}