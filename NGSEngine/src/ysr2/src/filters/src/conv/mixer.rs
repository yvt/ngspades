//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::cmp::min;
use std::borrow::Borrow;
use std::ops::Range;
use std::collections::BTreeMap;
use std::mem::swap;

use ysr2_common::stream::Generator;
use ysr2_common::dispatch::Queue;
use ysr2_common::utils::spectrum_convolve_additive;

use conv::{IrSpectrum, ConvSetup, ConvEnv};
use conv::source::Source;

/// Real-time convolution engine optimized for a large number of sources and
/// distinct impulse responses.
///
/// # Concepts
///
///  - **Sources** are signal sources. Duh.
///  - Each **mapping** represents a convolution of a source and `IrSpectrum`,
///    and also contains a channel index to write its output to.
///
/// # Optimizations
///
/// This implementation makes use of the following performance optimizations:
///
///  - A time distribution scheme (described below) is employed to uniform the
///    execution time of each call to `render`.
///  - Inactive sources (i.e. those with `Generator::is_active() == false`) and
///    mappings are excluded from the computation.
///
/// ## Time-Distributing Register: A Method for Uniform Execution Time Scheduling on Large Partitions
///
/// This convolution engine attempts to amortize the cost of large partitions by
/// employing the time-distribution scheme described below.
///
/// First, we move the location of some of frequency-domain delay units by
/// applying a method similar to [the register retiming].
///
/// [the register retiming]: https://en.wikipedia.org/wiki/Retiming
///
/// For example, the following circuit:
///
/// ```text
///     Generator --> FFT --> FDL --> (X) --> (+) --> IFFT
///                            |               A
///                            V               |
///                           FDL --> (X) -----+
/// ```
///
/// can be modified to this one:
///
/// ```text
///     Generator --> FDL --> FFT --> (X) --> (+) --> IFFT
///                            |               A
///                            V               |
///                           FDL --> (X) -----+
/// ```
///
/// or to this one:
///
/// ```text
///     Generator --> FFT --> (X) --> FDL --> (+) --> IFFT
///                    |                       A
///                    V                       |
///                   FDL --> (X) --> FDL -----+
/// ```
///
/// If we could find a subcircuit of the following form, the output of this
/// filter can be computed independently from the global block processing period.
///
/// ```text
///     --> Filter --> FDL -->
/// ```
///
/// For example, in the last example, multiplication operations (denoted by
/// `(X)`) now can run independently from the global block processing period,
/// on which the final block operation `IFFT` is executed. As a result we can
/// amortize the cost of processing large blocks provided a sufficient number of
/// sources to process.
///
/// We apply this method, which we call TDR (Time-Distributing Registers), in
/// the following situations:
///
///  - For block groups with FDLs (frequency-domain delay lines) with
///    lengths of one or more blocks, we apply this on input FFTs.
///  - For block groups with FDLs with lengths of two or more blocks,
///    we apply this on frequency domain operations.
///  - Amortization is not performed on output transforms since the number of
///    the operations does not increase with those of sources or mappings.
///
/// The lengths of FDLs are computed from `offset[i]` (see the documentation
/// of [`ConvParams`] for the definition) using the following formula:
/// `offset[i] / (1 << blocks[i].0)`. Therefore, it is important to configure
/// the number of blocks (`blocks[i].1`) and `latency` large enough so every
/// FDLs are at least two blocks long.
///
/// [`ConvParams`]: struct.ConvParams.html
///
/// # Hot-swapping Impulse Responses
///
/// This engine supports hot-swapping impulse responses i.e. replacing an IR of
/// a mapping without introducing audible noises by using the cross-fade
/// technique.
///
/// TODO: implement this feature
///
#[derive(Debug)]
pub struct MultiConvolver<T, I, Q> {
    queue: Q,
    setup: ConvSetup,
    num_outputs: usize,
    sources: BTreeMap<SourceId, McSource<T>>,
    mappings: BTreeMap<MappingId, Mapping<I>>,
    position: usize,
    next_source_id: u64,
    next_mapping_id: u64,
    groups: Vec<BlockGroupState>,
    output_buffer: Vec<Vec<f32>>,
    env: ConvEnv,
}

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
pub struct SourceId(u64);

#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Copy, Clone)]
pub struct MappingId(u64);

#[derive(Debug)]
pub struct SourceBuilder<'a, T: 'a, I: 'a, Q: 'a> {
    parent: &'a mut MultiConvolver<T, I, Q>,
    generator: T,
    num_channels: usize,
}

#[derive(Debug)]
pub struct MappingBuilder<'a, T: 'a, I: 'a, Q: 'a> {
    parent: &'a mut MultiConvolver<T, I, Q>,
    source_id: SourceId,
    ir: I,
    in_channel: usize,
    out_channel: usize,
    gain: f32,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum MultiConvolverMappingError {
    InvalidSourceId,
    InvalidSetup,
}

#[derive(Debug)]
struct Mapping<I> {
    ir: I,
    source_id: SourceId,
    output: usize,
    gain: f32,
}

#[derive(Debug)]
struct McSource<T> {
    source: Source,
    generator: T,
}

#[derive(Debug, Clone)]
struct BlockGroupState {
    next_tdr_source_id: Option<SourceId>,
    next_tdr_mapping_id: Option<MappingId>,

    preoutput_buffer: Vec<Vec<f32>>,
}

impl<T: Generator, I: Borrow<IrSpectrum>, Q: Queue> MultiConvolver<T, I, Q> {
    pub fn new(setup: &ConvSetup, num_output_channels: usize, queue: Q) -> Self {
        let max_block_size = 1usize << setup.params().blocks.last().unwrap().0;

        Self {
            queue,
            setup: setup.clone(),
            num_outputs: num_output_channels,
            sources: BTreeMap::new(),
            mappings: BTreeMap::new(),
            position: 0,
            next_source_id: 0,
            next_mapping_id: 0,
            groups: setup
                .params()
                .blocks
                .iter()
                .map(|&(size_log2, _)| {
                    let block_size = 1usize << size_log2;
                    BlockGroupState {
                        next_tdr_source_id: Some(SourceId(0)),
                        next_tdr_mapping_id: Some(MappingId(0)),

                        preoutput_buffer: vec![vec![0.0; block_size * 2]; num_output_channels],
                    }
                })
                .collect(),
            env: ConvEnv::new(setup),
            output_buffer: vec![vec![0.0; max_block_size * 2]; num_output_channels],
        }
    }

    /// Constructs `SourceBuilder` to be used to insert a source into this
    /// `MultiConvolver`.
    pub fn build_source(&mut self, generator: T) -> SourceBuilder<T, I, Q> {
        SourceBuilder {
            parent: self,
            generator,
            num_channels: 1,
        }
    }

    /// Constructs `SourceBuilder` to be used to insert a mapping into this
    /// `MultiConvolver`.
    pub fn build_mapping(&mut self, source_id: &SourceId, ir: I) -> MappingBuilder<T, I, Q> {
        MappingBuilder {
            parent: self,
            source_id: *source_id,
            ir,
            in_channel: 0,
            out_channel: 0,
            gain: 1.0,
        }
    }

    /// Retrieve a reference to the generator associated with the specified
    /// source.
    pub fn get_source_generator(&self, source_id: &SourceId) -> Option<&T> {
        self.sources.get(source_id).map(|x| &x.generator)
    }

    /// Retrieve a mutable reference to the generator associated with the
    /// specified source.
    pub fn get_source_generator_mut(&mut self, source_id: &SourceId) -> Option<&mut T> {
        self.sources.get_mut(source_id).map(|x| &mut x.generator)
    }

    /// Retrieve a reference to the `I` (used to borrow a `IrSpectrum`)
    /// associated with the specified mapping.
    pub fn get_mapping_ir(&self, mapping_id: &MappingId) -> Option<&I> {
        self.mappings.get(mapping_id).map(|x| &x.ir)
    }

    /// Remove a source and return the associated `Generator`.
    ///
    /// All mappings associated with the source must be removed before
    /// the following call to `render`.
    pub fn remove_source(&mut self, source_id: &SourceId) -> Option<T> {
        self.sources.remove(source_id).map(|x| x.generator)
    }

    /// Remove a mapping and return the associated `IrSpectrum`.
    ///
    /// Mathematically, this behaves like a multiplication on the *result* of
    /// the convolution with the function `1 - step(t - T)`. Therefore, an
    /// abrupt interruption of the reverbration sound can be heard if a mapping
    /// was removed when it is still active.
    pub fn remove_mapping(&mut self, mapping_id: &MappingId) -> Option<I> {
        self.mappings.remove(mapping_id).map(|x| x.ir)
    }
}

impl<'a, T: 'a, I: 'a, Q: 'a> SourceBuilder<'a, T, I, Q> {
    /// Set the number of input channels.
    ///
    /// Currently, the number of input channels must be exactly `1`.
    pub fn num_channels(mut self, num_channels: usize) -> Self {
        if num_channels != 1 {
            unimplemented!();
        }
        self.num_channels = num_channels;
        self
    }

    /// Insert a source to the `MultiConvolver` this `SourceBuilder` was
    /// created from.
    pub fn insert(self) -> SourceId {
        let mc = self.parent;
        let id = SourceId(mc.next_source_id);
        mc.next_source_id = mc.next_source_id.checked_add(1).unwrap();
        assert!(
            mc.sources
                .insert(
                    id,
                    McSource {
                        source: Source::new(&mc.setup),
                        generator: self.generator,
                    },
                )
                .is_none()
        );
        id
    }
}

impl<'a, T: 'a, I: 'a, Q: 'a> MappingBuilder<'a, T, I, Q> {
    /// Set the input (source) channel index. Defaults to `0`.
    ///
    /// The input channel index must be less than the number of input channels,
    /// which is specified by `SourceBuilder::num_channels`.
    pub fn in_channel(mut self, channel_index: usize) -> Self {
        if channel_index != 0 {
            unimplemented!();
        }
        self.in_channel = channel_index;
        self
    }

    /// Set the output channel index. Defaults to `0`.
    ///
    /// The output channel index must be less than the number of output
    /// channels.
    pub fn out_channel(mut self, channel_index: usize) -> Self {
        assert!(
            channel_index < self.parent.num_outputs,
            "channel index out of bounds"
        );
        self.in_channel = channel_index;
        self
    }

    /// Set the gain. Defaults to `1.0`.
    pub fn gain(mut self, gain: f32) -> Self {
        self.gain = gain;
        self
    }

    /// Insert a mapping to the `MultiConvolver` this `MappingBuilder` was
    /// created from.
    pub fn insert(self) -> Result<MappingId, MultiConvolverMappingError> {
        let mc = self.parent;
        if mc.sources.get(&self.source_id).is_none() {
            return Err(MultiConvolverMappingError::InvalidSourceId);
        }

        // TODO: check setup?

        let id = MappingId(mc.next_mapping_id);
        mc.next_mapping_id = mc.next_mapping_id.checked_add(1).unwrap();
        mc.mappings.insert(
            id,
            Mapping {
                ir: self.ir,
                source_id: self.source_id,
                // TODO: in_channel
                output: self.out_channel,
                gain: self.gain,
            },
        );

        Ok(id)
    }
}

impl<T: Generator, I: Borrow<IrSpectrum>, Q: Queue> Generator for MultiConvolver<T, I, Q> {
    fn render(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        // Fail-fast
        assert_eq!(to.len(), self.num_outputs);
        for ch in to.iter() {
            &ch[range.clone()];
        }

        let ref setup = self.setup;
        let ref mut group_states = self.groups;
        let min_block_size = 1usize << setup.params().blocks.first().unwrap().0;
        let max_block_size = 1usize << setup.params().blocks.last().unwrap().0;

        // Ordered by `SourceId.0` or `MappingId.0`
        let mut sources: Vec<_> = self.sources
            .iter_mut()
            .filter_map(|(source_id, source)| {
                let is_active = source.generator.is_active() ||
                    source.source.groups.iter().any(|g| g.num_active_blocks > 0);
                if is_active {
                    Some((source_id, source))
                } else {
                    source.generator.skip(range.len());
                    None
                }
            })
            .collect();
        let mut mappings: Vec<_> = self.mappings
            .iter_mut()
            .filter_map(|(mapping_id, mapping)| {
                // Choose only the mappings with active sources
                sources
                    .binary_search_by(|probe| probe.0.cmp(&mapping.source_id))
                    .ok()
                    .map(|i| (mapping_id, mapping, i))
            })
            .collect();

        let mut source_i_start: Vec<_> = group_states
            .iter()
            .map(|group_state| {
                group_state
                    .next_tdr_source_id
                    .map(|id| {
                        sources
                            .binary_search_by(|probe| probe.0.cmp(&id))
                            .unwrap_or_else(|i| i)
                    })
                    .unwrap_or(sources.len())
            })
            .collect();
        let mut mapping_i_start: Vec<_> = group_states
            .iter()
            .map(|group_state| {
                group_state
                    .next_tdr_mapping_id
                    .map(|id| {
                        mappings
                            .binary_search_by(|probe| probe.0.cmp(&id))
                            .unwrap_or_else(|i| i)
                    })
                    .unwrap_or(mappings.len())
            })
            .collect();

        let mut position = self.position;
        let mut index = range.start;

        // TODO: use `self.queue` for parallelization

        while index < range.end {
            // The number of samples to process in this iteration
            let num_processed_ub = range.end - index;

            // Bound `num_processed_ub` by the minimum block size boundary
            let min_block_pos = position & (min_block_size - 1);
            let num_processed = min(num_processed_ub, min_block_size - min_block_pos);
            let end_pos = position + num_processed;

            // Per-block block operations with TDRs
            for i in 0..group_states.len() {
                // Use the unbounded `num_processed_ub` since there is no point in
                // doing time-distribution thing *within* a single function call
                let block_size_log2 = setup.params().blocks[i].0;
                let block_size = 1usize << block_size_log2;
                let block_pos = position & (block_size - 1);
                let block_end_pos = block_pos + num_processed_ub;
                let block_end_pos_sat = min(block_end_pos, block_size);

                let group_info = setup.group(i);

                let source_i_end = mul_usize_x(block_end_pos_sat, sources.len(), block_size_log2);
                let mapping_i_end = mul_usize_x(block_end_pos_sat, mappings.len(), block_size_log2);

                if group_info.use_tdr_on_source {
                    for &mut (_, ref mut source) in &mut sources[source_i_start[i]..source_i_end] {
                        let ref mut src_group = source.source.groups[i];
                        if src_group.fresh_block.active {
                            for x in src_group.fresh_block.buffer.iter_mut() {
                                *x = 0.0;
                            }
                            src_group.fresh_block.active = false;
                        }

                        let first_block = src_group.blocks.front_mut().unwrap();
                        if first_block.active {
                            self.env.fft_envs[i][0].transform(first_block.buffer.as_mut_slice());
                        }
                    }
                }
                if group_info.use_tdr_on_mapping {
                    // Merge outputs from sources in the frequency domain
                    let ref mut preoutput_buffers = group_states[i].preoutput_buffer;
                    for &mut (_, ref mut mapping, source_i) in
                        &mut mappings[mapping_i_start[i]..mapping_i_end]
                    {
                        let ref source: Source = sources[source_i].1.source;
                        let ir: &IrSpectrum = mapping.ir.borrow();
                        for k in 0..ir.num_blocks_for_size(i) {
                            let ref block = source.groups[i].blocks[k + group_info.input_fdl_delay];
                            if !block.active {
                                continue;
                            }

                            spectrum_convolve_additive(
                                preoutput_buffers[mapping.output].as_mut_slice(),
                                &block.buffer,
                                ir.get(i, k),
                                mapping.gain,
                            );
                        }
                    }
                }

                if block_pos + num_processed == block_size {
                    // The processings required for this single cycle for this
                    // block size are all done
                    source_i_start[i] = 0;
                    mapping_i_start[i] = 0;
                } else {
                    source_i_start[i] = source_i_end;
                    mapping_i_start[i] = mapping_i_end;
                }
            }

            // Inject inputs
            for &mut (_, ref mut source) in sources.iter_mut() {
                source.source.feed(position..end_pos, &mut source.generator);
            }

            // Write outputs
            let ob_pos = position & (max_block_size * 2 - 1);
            for (to, ob) in to.iter_mut().zip(self.output_buffer.iter()) {
                if ob_pos + num_processed > ob.len() {
                    let n = ob.len() - ob_pos;
                    to[index..index + n].copy_from_slice(&ob[ob_pos..]);
                    to[index + n..index + num_processed].copy_from_slice(&ob[0..num_processed - n]);
                } else {
                    to[index..index + num_processed].copy_from_slice(
                        &ob[ob_pos..
                                ob_pos +
                                    num_processed],
                    );
                }
            }

            // Per-group block processsing (possibly without TDRs)
            let ob_pos = (position + num_processed) & (max_block_size * 2 - 1);
            for (i, group_state) in group_states.iter_mut().enumerate().rev() {
                let is_largest = (i + 1) == setup.groups.len();
                let group_info = setup.group(i);
                let block_size_log2 = setup.params().blocks[i].0;
                let block_size = 1usize << block_size_log2;
                if end_pos & (block_size - 1) != 0 {
                    continue;
                }

                for &mut (_, ref mut source) in sources.iter_mut() {
                    let ref mut src_group = source.source.groups[i];
                    let mut tmp = src_group.blocks.pop_back().unwrap();

                    if tmp.active {
                        src_group.num_active_blocks -= 1;
                    }

                    if group_info.use_tdr_on_source {
                        // Let TDR routine do the (expensive) zero-fill
                        swap(&mut tmp, &mut src_group.fresh_block);
                        swap(&mut tmp, &mut src_group.next_block);
                    } else {
                        self.env.fft_envs[i][0].transform(
                            src_group
                                .next_block
                                .buffer
                                .as_mut_slice(),
                        );
                        tmp.active = false;
                        for x in tmp.buffer.iter_mut() {
                            *x = 0.0;
                        }
                        swap(&mut tmp, &mut src_group.next_block);
                    }
                    src_group.blocks.push_front(tmp);
                }

                // Merge outputs from all sources in the frequency domain
                let ref mut preoutput_buffers = group_state.preoutput_buffer;
                if !self.setup.group(i).use_tdr_on_mapping {
                    for &mut (_, ref mut mapping, source_i) in mappings.iter_mut() {
                        let ref source: Source = sources[source_i].1.source;
                        let ir: &IrSpectrum = mapping.ir.borrow();
                        for k in 0..ir.num_blocks_for_size(i) {
                            let ref block = source.groups[i].blocks[k + group_info.input_fdl_delay];
                            if !block.active {
                                continue;
                            }

                            spectrum_convolve_additive(
                                preoutput_buffers[mapping.output].as_mut_slice(),
                                &block.buffer,
                                ir.get(i, k),
                                mapping.gain,
                            );
                        }
                    }
                }

                // Apply the inverse FFT and update the output buffer
                for (ch, ob) in preoutput_buffers.iter_mut().zip(
                    self.output_buffer.iter_mut(),
                )
                {
                    self.env.fft_envs[i][1].transform(ch.as_mut_slice());
                    if is_largest {
                        assert_eq!(ob.len(), ch.len());
                        let half = ob.len() / 2;
                        if ob_pos == 0 {
                            for i in 0..half {
                                ob[i] += ch[i];
                            }
                            for i in half..ob.len() {
                                ob[i] = ch[i];
                            }
                        } else if ob_pos == half {
                            for i in 0..half {
                                ob[i] = ch[i + half];
                            }
                            for i in 0..half {
                                ob[i + half] += ch[i];
                            }
                        } else {
                            unreachable!();
                        }
                    } else {
                        let ob_len = ob.len();
                        let half = ch.len() / 2;
                        for (x, y) in ch[0..half].iter().zip(ob[ob_pos..].iter_mut()) {
                            *y += *x;
                        }
                        for (x, y) in ch[half..].iter().zip(
                            ob[(ob_pos + half) & (ob_len - 1)..]
                                .iter_mut(),
                        )
                        {
                            *y += *x;
                        }
                    }

                    for x in ch.iter_mut() {
                        *x = 0.0;
                    }
                }
            }

            index += num_processed;
            position = position.wrapping_add(num_processed);
        }

        self.position = position;

        for (group_state, &i) in group_states.iter_mut().zip(source_i_start.iter()) {
            group_state.next_tdr_source_id = sources.get(i).map(|s| *s.0);
        }
        for (group_state, &i) in group_states.iter_mut().zip(mapping_i_start.iter()) {
            group_state.next_tdr_mapping_id = mappings.get(i).map(|s| *s.0);
        }
    }

    fn skip(&mut self, mut num_samples: usize) {
        if self.is_active() {
            let mut buf = vec![vec![0.0; 1024]; self.num_outputs];
            let mut buf_slices = buf.iter_mut().map(Vec::as_mut_slice).collect::<Vec<_>>();
            while self.is_active() && num_samples > 0 {
                let processed = if num_samples > 1024 {
                    1024
                } else {
                    num_samples
                };
                for ch in buf_slices.iter_mut() {
                    for x in ch.iter_mut() {
                        *x = 0.0;
                    }
                }
                self.render(&mut buf_slices, 0..processed);
                num_samples -= processed;
            }
        }

        if num_samples > 0 {
            for (_, source) in self.sources.iter_mut() {
                source.generator.skip(num_samples);
            }
        }
    }

    fn is_active(&self) -> bool {
        // TODO: `MultiConvolver::is_active`
        true
    }
}

// Fixed-point multiplication in `usize`
fn mul_usize_x(x: usize, y: usize, fract: u32) -> usize {
    ((x as u64).checked_mul(y as u64).unwrap() >> fract) as usize
}
