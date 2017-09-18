//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::collections::VecDeque;
use std::ops::Range;

use ysr2_common::stream::Generator;

use conv::ConvSetup;

// TODO: optimize the cases where `Generator` is inactive

#[derive(Debug)]
pub struct Source {
    pub groups: Vec<SourceBlockGroup>,
}

#[derive(Debug)]
pub struct SourceBlockGroup {
    pub blocks: VecDeque<SourceBlock>,
    pub next_block: SourceBlock,

    /// The number of non-zero blocks in `blocks` and `next_block`.
    pub num_active_blocks: usize,

    /// Used only if TDR is enabled
    pub fresh_block: SourceBlock,
}

#[derive(Debug)]
pub struct SourceBlock {
    /// `Vec` of the length twice as large as the block size.
    pub buffer: Vec<f32>,

    pub active: bool,
}

impl Source {
    pub fn new(setup: &ConvSetup) -> Self {
        let params = setup.params();
        Self {
            groups: params
                .blocks
                .iter()
                .enumerate()
                .map(|(i, &(size_log2, num_blocks))| {
                    let size2 = 2 << size_log2;
                    let delay = setup.group(i).input_fdl_delay;
                    SourceBlockGroup {
                        blocks: (0..(num_blocks + delay))
                            .map(|_| {
                                SourceBlock {
                                    buffer: vec![0.0; size2],
                                    active: false,
                                }
                            })
                            .collect(),
                        next_block: SourceBlock {
                            buffer: vec![0.0; size2],
                            active: false,
                        },
                        fresh_block: SourceBlock {
                            buffer: vec![
                                0.0;
                                if setup.group(i).use_tdr_on_source {
                                    size2
                                } else {
                                    0
                                }
                            ],
                            active: false,
                        },
                        num_active_blocks: 0,
                    }
                })
                .collect(),
        }
    }

    /// Fill `next_block`s with the output from the given `Generator`.
    ///
    /// `range` must not span across a minimum block size boundary.
    pub fn feed<T: Generator>(&mut self, range: Range<usize>, gen: &mut T) {
        if range.start >= range.end {
            return;
        }

        // `range` mustn't span across a frame boundary
        let min_bs = self.groups[0].next_block.buffer.len() / 2;
        assert_eq!(range.start & !(min_bs - 1), (range.end - 1) & !(min_bs - 1));

        let is_active = gen.is_active();

        // Generate on the largest `next_block`
        let (last, rest) = self.groups.split_last_mut().unwrap();
        let last_bs = last.next_block.buffer.len() / 2;
        let last_range = range.start & (last_bs - 1)..((range.end - 1) & (last_bs - 1)) + 1;
        gen.render(&mut [&mut last.next_block.buffer[..]], last_range.clone());

        if is_active && !last.next_block.active {
            last.next_block.active = true;
            last.num_active_blocks += 1;
        }

        // And then copy the result to smaller ones
        for partition in rest.iter_mut() {
            let p_bs = partition.next_block.buffer.len() / 2;
            let p_range = range.start & (p_bs - 1)..((range.end - 1) & (p_bs - 1)) + 1;
            partition.next_block.buffer[p_range].copy_from_slice(
                &last.next_block.buffer
                    [last_range.clone()],
            );

            if is_active && !partition.next_block.active {
                partition.next_block.active = true;
                partition.num_active_blocks += 1;
            }
        }
    }
}
