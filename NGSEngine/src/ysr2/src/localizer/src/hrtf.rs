//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::cmp::min;
use std::{mem, fmt};
use std::ops::Range;
use std::cell::RefCell;
use cgmath::Vector3;
use std::collections::HashMap;

use yfft;

use ysr2_common::stream::Generator;
use ysr2_common::dispatch::Queue;
use ysr2_common::values::DynamicSlerpVector3;
use ysr2_kemar_data::{Sample, KEMAR_DATA};

use Panner;

lazy_static! {
    static ref BIN_TABLE: BinTable = BinTable::new();
}

#[derive(Debug)]
struct BinTable {
    bins: Vec<BinInfo>,
    rings: Vec<Vec<usize>>,
}

#[derive(Debug)]
struct BinInfo {
    sample: &'static Sample,
    reversed: bool,
    azimuth: i32,
}

impl BinTable {
    fn new() -> Self {
        let mut bins = Vec::new();
        let rings: Vec<_> = KEMAR_DATA
            .iter()
            .enumerate()
            .map(|(i, ring)| {
                let samples = ring.samples;
                assert_eq!(ring.elevation, i as i32 * 10 - 40);

                let mut bin_indices = Vec::new();

                // left side
                for sample in samples.iter().rev() {
                    if sample.azimuth == 0 {
                        continue;
                    }
                    bin_indices.push(bins.len());
                    bins.push(BinInfo {
                        sample,
                        reversed: true,
                        azimuth: -sample.azimuth,
                    });
                }

                // right side
                for sample in samples.iter() {
                    if sample.azimuth == 180 {
                        continue;
                    }
                    bin_indices.push(bins.len());
                    bins.push(BinInfo {
                        sample,
                        reversed: false,
                        azimuth: sample.azimuth,
                    });
                }
                if samples.last().unwrap().azimuth == 180 {
                    // Shared the same bin
                    let i = bin_indices[0];
                    bin_indices.push(i);
                }

                bin_indices
            })
            .collect();

        assert_eq!(rings.len(), 14);

        Self { bins, rings }
    }

    fn find_bin(&self, v: Vector3<f32>) -> usize {
        let cyl_r = (v.x * v.x + v.z * v.z).sqrt();

        // Compute the elevation. Since we don't have response data for
        // elevations lower than -40deg, clamp to -40deg
        let elevation = (v.y / (cyl_r + ::std::f32::EPSILON)).atan().to_degrees();
        let ring_i = (elevation * 0.1 + 4.0).max(0.0).round() as usize;

        // Retrieve the ring info
        let ring_info: &[usize] = &self.rings[ring_i];

        // Choose the bin
        if ring_info.len() == 1 {
            ring_info[0]
        } else {
            let az = v.x.atan2(v.z).to_degrees() as i32;
            match ring_info.binary_search_by(|i| self.bins[*i].azimuth.cmp(&az)) {
                Ok(ri_i) => ring_info[ri_i],
                Err(ri_i) => {
                    // Find a nearest neighbor
                    if ri_i == 0 {
                        *ring_info.first().unwrap()
                    } else if ri_i == ring_info.len() {
                        *ring_info.last().unwrap()
                    } else {
                        let bi1 = ring_info[ri_i - 1];
                        let bi2 = ring_info[ri_i];
                        let az1 = self.bins[bi1].azimuth;
                        let az2 = self.bins[bi2].azimuth;
                        let mid = (az1 + az2) >> 1;
                        if az >= mid { bi2 } else { bi1 }
                    }
                }
            }
        }
    }

    fn find_bin_f64(&self, v: Vector3<f64>) -> usize {
        self.find_bin(Vector3::new(v.x as f32, v.y as f32, v.z as f32))
    }
}

lazy_static! {
    static ref YFFT_SETUPS: [yfft::Setup<f32>; 2] = {
        let setup1 = yfft::Setup::new(&yfft::Options {
            input_data_order: yfft::DataOrder::Natural,
            output_data_order: yfft::DataOrder::Natural,
            input_data_format: yfft::DataFormat::Real,
            output_data_format: yfft::DataFormat::HalfComplex,
            len: 256,
            inverse: false,
        }).unwrap();
        let setup2 = yfft::Setup::new(&yfft::Options {
            input_data_order: yfft::DataOrder::Natural,
            output_data_order: yfft::DataOrder::Natural,
            input_data_format: yfft::DataFormat::HalfComplex,
            output_data_format: yfft::DataFormat::Real,
            len: 256,
            inverse: true,
        }).unwrap();
        [setup1, setup2]
    };
}

thread_local! {
    static YFFT_ENVS: RefCell<[yfft::Env<f32, &'static yfft::Setup<f32>>; 2]> =
        RefCell::new([
            yfft::Env::new(&YFFT_SETUPS[0]),
            yfft::Env::new(&YFFT_SETUPS[1]),
        ]);
}

/// A panner based on the pinna (HRTF) model.
///
/// This has the following restrictions:
///
///  - The input/output sampling rate is fixed at 44100 hertz.
///  - The output channel configuration must be `Stereo`.
///  - Has the inherent latency of 192 samples (128 for FFT-based convolution,
///    64 from the HRTF data set).
///  - The processing done using the 128-sample blocks. Because of this, the
///    processing latency varies disruptively if the output buffer size is not
///    a multiple of 128.
///
pub struct HrtfPanner<T: Generator, Q: Queue> {
    queue: Q,
    bins: PannerBinTable,

    sources: HashMap<SourceId, Source<T>>,

    output_buffer: Box<[[f32; 128]; 2]>,
    carry_buffer: Box<[[f32; 128]; 2]>,
    output_active: bool,
    carry_active: bool,

    // Only used during rendering
    accum_buffer: Vec<[[f32; 256]; 2]>,

    next_src_id: u64,

    /// [0, 127]
    position: usize,
}

impl<T: Generator, Q: Queue> fmt::Debug for HrtfPanner<T, Q>
where
    T: fmt::Debug,
    Q: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("HrtfPanner")
            .field("queue", &self.queue)
            .field("sources", &self.sources)
            .field("next_src_id", &self.next_src_id)
            .field("position", &self.position)
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct SourceId(u64);

struct Source<T: Generator> {
    generator: T,
    buffer: Box<[f32; 128]>,
    direction: DynamicSlerpVector3,
}

impl<T: Generator> fmt::Debug for Source<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Source")
            .field("generator", &self.generator)
            .field("direction", &self.direction)
            .finish()
    }
}

struct PannerBinTable {
    bins: Vec<Bin>,
    bin_table: &'static BinTable,
    feed_bins: Vec<usize>,
    active_bins: Vec<usize>,
}

#[derive(Clone)]
struct Bin {
    buffer: [[f32; 256]; 2],
    first_feed_source: [Option<usize>; 3],
    active: bool,
}

impl<T: Generator, Q: Queue> HrtfPanner<T, Q> {
    pub fn new(queue: Q) -> Self {
        let bin_table: &BinTable = &*BIN_TABLE;
        let num_accum = queue.hardware_concurrency();

        let bin = Bin {
            buffer: [[0.0; 256], [0.0; 256]],
            first_feed_source: [None, None, None],
            active: false,
        };
        Self {
            queue,
            bins: PannerBinTable {
                bins: vec![bin; bin_table.bins.len()],
                bin_table,
                active_bins: Vec::new(),
                feed_bins: Vec::new(),
            },
            output_buffer: Box::new([[0.0; 128], [0.0; 128]]),
            output_active: false,
            carry_buffer: Box::new([[0.0; 128], [0.0; 128]]),
            carry_active: false,
            accum_buffer: vec![[[0.0; 256], [0.0; 256]]; num_accum],
            next_src_id: 0,
            position: 0,
            sources: HashMap::new(),
        }
    }
}

impl<T: Generator + Send + Sync, Q: Queue> Panner<T> for HrtfPanner<T, Q> {
    type SourceId = SourceId;

    fn insert(&mut self, generator: T) -> SourceId {
        let new_next_id = self.next_src_id.checked_add(1).unwrap();
        let id = SourceId(self.next_src_id);

        self.sources.insert(
            id,
            Source {
                generator,
                buffer: Box::new([0.0; 128]),
                direction: DynamicSlerpVector3::new(Vector3::unit_z()),
            },
        );

        self.next_src_id = new_next_id;
        id
    }

    fn generator(&self, id: &SourceId) -> Option<&T> {
        self.sources.get(id).map(|source| &source.generator)
    }

    fn generator_mut(&mut self, id: &SourceId) -> Option<&mut T> {
        self.sources.get_mut(id).map(|source| &mut source.generator)
    }

    fn direction(&self, id: &SourceId) -> Option<&DynamicSlerpVector3> {
        self.sources.get(id).map(|source| &source.direction)
    }

    fn direction_mut(&mut self, id: &SourceId) -> Option<&mut DynamicSlerpVector3> {
        self.sources.get_mut(id).map(|source| &mut source.direction)
    }

    fn remove(&mut self, id: &SourceId) -> Option<T> {
        self.sources.remove(id).map(|source| source.generator)
    }
}

impl PannerBinTable {
    fn add_active_bin<'a>(&mut self, bin_i: usize) {
        let ref mut bin = self.bins[bin_i];
        if !bin.active {
            bin.active = true;
            self.active_bins.push(bin_i);
        }
    }

    fn add_feed_bin<'a>(&mut self, bin_i: usize) {
        let ref bin = self.bins[bin_i];
        if bin.first_feed_source[0].is_none() && bin.first_feed_source[1].is_none() &&
            bin.first_feed_source[2].is_none()
        {
            self.feed_bins.push(bin_i);
        }
    }

    #[cfg(debug_assertions)]
    fn validate_active_bins(&mut self) {
        // The order does not matter
        self.active_bins.sort_unstable();

        if self.active_bins.len() > 0 {
            // Check out-of-bounds
            assert!(*self.active_bins.last().unwrap() < self.bins.len());

            // You must not form multiple mutable reference to a single object.
            // Otherwise, an undefined behaviour would occur
            for i in 1..self.active_bins.len() {
                assert_ne!(
                    self.active_bins[i - 1],
                    self.active_bins[i],
                    "active_bins contains duplicate elements: {:?}",
                    self.active_bins
                );
            }
        }
    }

    #[cfg(not(debug_assertions))]
    fn validate_active_bins(&self) {}

    #[cfg(debug_assertions)]
    fn validate_feed_bins(&mut self) {
        // The order does not matter
        self.feed_bins.sort_unstable();

        if self.feed_bins.len() > 0 {
            // Check out-of-bounds
            assert!(*self.feed_bins.last().unwrap() < self.bins.len());

            // You must not form multiple mutable reference to a single object.
            // Otherwise, an undefined behaviour would occur
            for i in 1..self.feed_bins.len() {
                assert_ne!(
                    self.feed_bins[i - 1],
                    self.feed_bins[i],
                    "feed_bins contains duplicate elements: {:?}",
                    self.feed_bins
                );
            }
        }
    }

    #[cfg(not(debug_assertions))]
    fn validate_feed_bins(&self) {}
}

#[derive(Debug)]
struct SourceInfo<'a, T: Generator + 'a> {
    source: &'a mut Source<T>,

    // Singly-linked list
    next: [Option<usize>; 3],
}

struct SendPtr<T>(*mut T);

unsafe impl<T> Sync for SendPtr<T> {}
unsafe impl<T> Send for SendPtr<T> {}

impl<T: Generator + Send + Sync, Q: Queue> Generator for HrtfPanner<T, Q> {
    fn render(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        assert_eq!(to.len(), 2, "output must be stereo");
        let (to1, to2) = to.split_at_mut(1);
        let (to1, to2) = (&mut to1[0], &mut to2[0]);

        // validate the range
        assert!(range.start <= range.end);
        let _ = (&to1[range.clone()], &to2[range.clone()]);

        let mut out_i = range.start;

        while out_i < range.end {
            let window_size = min(range.end - out_i, 128 - self.position);
            let next_out_i = out_i + window_size;

            let chunk_i = self.position;
            let end_chunk_i = self.position + window_size;

            // Write the output
            to1[out_i..next_out_i].copy_from_slice(&self.output_buffer[0][chunk_i..end_chunk_i]);
            to2[out_i..next_out_i].copy_from_slice(&self.output_buffer[1][chunk_i..end_chunk_i]);

            // Collect the active sources
            let mut sources = Vec::with_capacity(self.sources.len());
            for (_, source) in self.sources.iter_mut() {
                if !source.generator.is_active() {
                    source.direction.update_multi(window_size as f64);
                    source.generator.skip(window_size);
                    continue;
                }

                let bin1 = self.bins.bin_table.find_bin_f64(source.direction.get());
                source.direction.update_multi(window_size as f64);
                let bin2 = self.bins.bin_table.find_bin_f64(source.direction.get());

                self.bins.add_active_bin(bin1);
                self.bins.add_feed_bin(bin1);

                let mut next = [None, None, None];

                if bin1 == bin2 {
                    next[2] = self.bins.bins[bin1].first_feed_source[2];
                    self.bins.bins[bin1].first_feed_source[2] = Some(sources.len());
                } else {
                    self.bins.add_feed_bin(bin2);
                    self.bins.add_active_bin(bin2);

                    next[0] = self.bins.bins[bin1].first_feed_source[0];
                    next[1] = self.bins.bins[bin2].first_feed_source[1];

                    self.bins.bins[bin1].first_feed_source[0] = Some(sources.len());
                    self.bins.bins[bin2].first_feed_source[1] = Some(sources.len());
                }

                sources.push(SourceInfo { source, next });
            }

            // Render the sources
            self.queue.foreach(&mut sources, |_, source_info| {
                let ref mut source = source_info.source;
                source.generator.render(
                    &mut [&mut *source.buffer],
                    chunk_i..end_chunk_i,
                );
            });

            let gain_start = [1.0, 0.0];
            let gain_step = [-1.0 / window_size as f32, 1.0 / window_size as f32];

            // Feed each bin (accumulate each bin's `buffer[0]`)
            //
            // The elements of `self.bins.bins` are accessed via the pointer
            // `bins`. Accessing them is an unsafe operation, which are
            // proven to safe. `validate_feed_bins` contains assertions
            // which should pass if the proof is valid but runtime checks
            // required to do such assertions are disabled in the release
            // build because their cost is not negligible.
            if self.bins.feed_bins.len() > 0 {
                self.bins.validate_feed_bins();
                let bins = SendPtr(self.bins.bins.as_mut_ptr());
                let ref feed_bins = self.bins.feed_bins;
                self.queue.apply(feed_bins.len(), |i| {
                    let bin: &mut Bin = unsafe { &mut *bins.0.offset(feed_bins[i] as isize) };

                    for x in bin.buffer[0][chunk_i..end_chunk_i].iter_mut() {
                        *x = 0.0;
                    }

                    for source_type in 0..3 {
                        let mut source_i_opt = bin.first_feed_source[source_type];
                        while let Some(source_i) = source_i_opt {
                            let ref source_info: SourceInfo<T> = sources[source_i];
                            let ref source = source_info.source;

                            let source_data = &source.buffer[0..end_chunk_i];
                            let bin_data = &mut bin.buffer[0][0..end_chunk_i];

                            if source_type == 2 {
                                for i in chunk_i..end_chunk_i {
                                    bin_data[i] += source_data[i];
                                }
                            } else {
                                let gain_start = gain_start[source_type];
                                let gain_step = gain_step[source_type];
                                let mut gain = gain_start;

                                for i in chunk_i..end_chunk_i {
                                    bin_data[i] += source_data[i] * gain;
                                    gain += gain_step;
                                }
                            }

                            source_i_opt = source_info.next[source_type];
                        }
                    }

                    bin.first_feed_source = [None, None, None];
                });
            }
            self.bins.feed_bins.clear();

            self.position += window_size;

            if self.position == 128 {
                // The current window has enough input data and is ready
                // to process
                self.position = 0;

                // Process each bin
                //
                // The elements of `self.bins.bins` are accessed via the pointer
                // `bins`. Accessing them is an unsafe operation, which are
                // proven to safe. `validate_active_bins` contains assertions
                // which should pass if the proof is valid but runtime checks
                // required to do such assertions are disabled in the release
                // build because their cost is not negligible.
                if self.bins.active_bins.len() > 0 {
                    self.bins.validate_active_bins();
                    let bins = SendPtr(self.bins.bins.as_mut_ptr());
                    let ref active_bins = self.bins.active_bins;
                    let bin_table = self.bins.bin_table;
                    self.queue.apply(active_bins.len(), |i| {
                        let bin: &mut Bin = unsafe { &mut *bins.0.offset(active_bins[i] as isize) };

                        for i in 0..128 {
                            bin.buffer[0][i] *= 1.0 / 128.0;
                            bin.buffer[1][i] = bin.buffer[0][i];
                        }
                        for i in 128..256 {
                            bin.buffer[0][i] = 0.0;
                            bin.buffer[1][i] = 0.0;
                        }

                        // forward FFT
                        YFFT_ENVS.with(|envs| {
                            let mut envs = envs.borrow_mut();

                            envs[0].transform(&mut bin.buffer[0]);
                            envs[0].transform(&mut bin.buffer[1]);

                            // perform convolution
                            let ref bin_info: BinInfo = bin_table.bins[active_bins[i]];

                            if bin_info.reversed {
                                spectrum_convolve(
                                    &mut bin.buffer[0],
                                    &bin_info.sample.ir_fft_hc[1],
                                );
                                spectrum_convolve(
                                    &mut bin.buffer[1],
                                    &bin_info.sample.ir_fft_hc[0],
                                );
                            } else {
                                spectrum_convolve(
                                    &mut bin.buffer[0],
                                    &bin_info.sample.ir_fft_hc[0],
                                );
                                spectrum_convolve(
                                    &mut bin.buffer[1],
                                    &bin_info.sample.ir_fft_hc[1],
                                );
                            }

                            // backward FFT
                            envs[1].transform(&mut bin.buffer[0]);
                            envs[1].transform(&mut bin.buffer[1]);
                        });

                        bin.active = false;
                    });
                }

                // Accumulate outputs from all active bins and then update
                // `output_buffer` and `carry_buffer`
                {
                    let ref bins = self.bins.bins;
                    let ref active_bins = self.bins.active_bins;
                    if active_bins.len() > self.accum_buffer.len() * 2 &&
                        self.accum_buffer.len() > 1
                    {
                        // Two-phase reduction
                        let num_accum_buffers = self.accum_buffer.len();
                        self.queue.foreach(
                            &mut self.accum_buffer,
                            |i, accum_buffer| {
                                let start_bin = active_bins.len() * i / num_accum_buffers;
                                let end_bin = active_bins.len() * (i + 1) / num_accum_buffers;
                                accum_buffer[0].copy_from_slice(
                                    &bins[active_bins[start_bin]].buffer[0],
                                );
                                accum_buffer[1].copy_from_slice(
                                    &bins[active_bins[start_bin]].buffer[1],
                                );
                                for bin_i in start_bin + 1..end_bin {
                                    let ref bin = bins[active_bins[bin_i]];
                                    for i in 0..256 {
                                        accum_buffer[0][i] += bin.buffer[0][i];
                                        accum_buffer[1][i] += bin.buffer[1][i];
                                    }
                                }
                            },
                        );

                        self.output_buffer[0].copy_from_slice(&self.accum_buffer[0][0][0..128]);
                        self.output_buffer[1].copy_from_slice(&self.accum_buffer[0][1][0..128]);

                        for ab_i in 1..self.accum_buffer.len() {
                            let ref accum_buffer = self.accum_buffer[ab_i];
                            for i in 0..128 {
                                self.output_buffer[0][i] += accum_buffer[0][i];
                                self.output_buffer[1][i] += accum_buffer[1][i];
                            }
                        }

                        for i in 0..128 {
                            self.output_buffer[0][i] += self.carry_buffer[0][i];
                            self.output_buffer[1][i] += self.carry_buffer[1][i];
                        }

                        self.carry_buffer[0].copy_from_slice(&self.accum_buffer[0][0][128..256]);
                        self.carry_buffer[1].copy_from_slice(&self.accum_buffer[0][1][128..256]);

                        for ab_i in 1..self.accum_buffer.len() {
                            let ref accum_buffer = self.accum_buffer[ab_i];
                            for i in 0..128 {
                                self.carry_buffer[0][i] += accum_buffer[0][i + 128];
                                self.carry_buffer[1][i] += accum_buffer[1][i + 128];
                            }
                        }

                        self.output_active = true;
                        self.carry_active = true;
                    } else {
                        // Simple reduction
                        if active_bins.len() > 0 {
                            self.output_buffer[0].copy_from_slice(
                                &bins[active_bins[0]].buffer[0][0..128],
                            );
                            self.output_buffer[1].copy_from_slice(
                                &bins[active_bins[0]].buffer[1][0..128],
                            );

                            for bin_i in 1..active_bins.len() {
                                let ref bin = bins[active_bins[bin_i]];
                                for i in 0..128 {
                                    self.output_buffer[0][i] += bin.buffer[0][i];
                                    self.output_buffer[1][i] += bin.buffer[1][i];
                                }
                            }

                            for i in 0..128 {
                                self.output_buffer[0][i] += self.carry_buffer[0][i];
                                self.output_buffer[1][i] += self.carry_buffer[1][i];
                            }

                            self.carry_buffer[0].copy_from_slice(
                                &bins[active_bins[0]].buffer[0][128..
                                                                    256],
                            );
                            self.carry_buffer[1].copy_from_slice(
                                &bins[active_bins[0]].buffer[1][128..
                                                                    256],
                            );

                            for bin_i in 1..active_bins.len() {
                                let ref bin = bins[active_bins[bin_i]];
                                for i in 0..128 {
                                    self.carry_buffer[0][i] += bin.buffer[0][i + 128];
                                    self.carry_buffer[1][i] += bin.buffer[1][i + 128];
                                }
                            }

                            self.output_active = true;
                            self.carry_active = true;
                        } else {
                            mem::swap(&mut self.carry_buffer, &mut self.output_buffer);

                            if self.output_active {
                                for i in 0..128 {
                                    self.carry_buffer[0][i] = 0.0;
                                    self.carry_buffer[1][i] = 0.0;
                                }
                            }

                            self.output_active = self.carry_active;
                            self.carry_active = false;
                        }
                    }
                }

                self.bins.active_bins.clear();
            }

            out_i = next_out_i;
        }
    }

    fn skip(&mut self, num_samples: usize) {
        for (_, source) in self.sources.iter_mut() {
            source.generator.skip(num_samples);
        }
    }

    fn is_active(&self) -> bool {
        for (_, source) in self.sources.iter() {
            if source.generator.is_active() {
                return true;
            }
        }
        self.bins.active_bins.len() > 0 || self.carry_active || self.output_active
    }
}

/// Perform convolution given two data serieses in the half complex format in
/// the frequency domain.
fn spectrum_convolve(buffer: &mut [f32; 256], ir_fq: &[f32; 256]) {
    // A (cyclic) convolution in the time domain can be accomplished by the
    // pointwise product in the frequency domain.
    buffer[0] *= ir_fq[0];
    buffer[1] *= ir_fq[1];
    for i in 1..128 {
        let (r1, i1) = (buffer[i * 2], buffer[i * 2 + 1]);
        let (r2, i2) = (ir_fq[i * 2], ir_fq[i * 2 + 1]);
        buffer[i * 2] = r1 * r2 - i1 * i2;
        buffer[i * 2 + 1] = r1 * i2 + r2 * i1;
    }
}
