//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use imp::{ResourceBinder, PipelineLayout, DescriptorSet};

// use std::default

pub const MAX_DESCRIPTOR_SETS: usize = 4;

#[derive(Debug)]
pub struct DescriptorSetBindingState {
    pipeline_layout: Option<PipelineLayout>,
    descriptor_sets: [Option<DescriptorSet>; MAX_DESCRIPTOR_SETS],
}

impl DescriptorSetBindingState {
    pub fn new() -> Self {
        Self {
            pipeline_layout: Default::default(),
            descriptor_sets: Default::default(),
        }
    }

    pub(crate) fn bind_descriptor_sets<T: ResourceBinder>(
        &mut self,
        binder: &T,
        pipeline_layout: &PipelineLayout,
        start_index: usize,
        descriptor_sets: &[&DescriptorSet],
        dynamic_offsets: &[u32],
    ) {
        if self.pipeline_layout.as_ref() != Some(pipeline_layout) {
            // Pipeline layout has changed; reset `descriptor_sets`
            self.pipeline_layout = Some(pipeline_layout.clone());
            self.descriptor_sets = Default::default();
        }

        assert!(
            start_index + descriptor_sets.len() <= MAX_DESCRIPTOR_SETS,
            "too many descriptor sets"
        );

        let mut dyn_off_idx = 0;
        for (i, &ds) in descriptor_sets.iter().enumerate() {
            let set_index = i + start_index;
            let num_dyn_offs = ds.layout().num_dynamic_offsets();
            let ref sub_dyn_offs = dynamic_offsets[dyn_off_idx..dyn_off_idx + num_dyn_offs];

            if Some(ds) == self.descriptor_sets[set_index].as_ref() {
                // only update dynamic offsets
                pipeline_layout.update_dynamic_offset(binder, set_index, ds, sub_dyn_offs);
            } else {
                // full update
                pipeline_layout.bind_descriptor_set(binder, set_index, ds, sub_dyn_offs);
                self.descriptor_sets[set_index] = Some(ds.clone());
            }

            dyn_off_idx += num_dyn_offs;
        }
    }
}
