//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::cmp;
use std::fmt;
use std::ops::Range;
use cgmath::Vector3;
use std::collections::HashMap;

use ysr2_common::stream::Generator;
use ysr2_common::dispatch::Queue;
use ysr2_common::values::DynamicSlerpVector3;

use Panner;

/// A panner using an equal-power panning algorithm.
///
/// This has the following restrictions:
///
///  - The output channel configuration must be `Stereo`.
///
pub struct EqualPowerPanner<T: Generator, Q: Queue> {
    queue: Q,

    sources: HashMap<SourceId, Source<T>>,

    // Only used during rendering
    accum_buffer: Vec<Vec<Vec<f32>>>,

    next_src_id: u64,
}

impl<T: Generator, Q: Queue> fmt::Debug for EqualPowerPanner<T, Q>
where
    T: fmt::Debug,
    Q: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("EqualPowerPanner")
            .field("queue", &self.queue)
            .field("sources", &self.sources)
            .field("next_src_id", &self.next_src_id)
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct SourceId(u64);

#[derive(Debug)]
struct Source<T> {
    generator: T,
    buffer: Vec<Vec<f32>>,
    direction: DynamicSlerpVector3,
}

impl<T> Source<T> {
    fn channel_gain(&self) -> [f32; 2] {
        let x = (self.direction.get().x as f32).max(-1.0).min(1.0) * 0.5;
        [(0.5 - x).sqrt(), (0.5 + x).sqrt()]
    }
}

impl<T: Generator, Q: Queue> EqualPowerPanner<T, Q> {
    pub fn new(queue: Q) -> Self {
        let num_accum = queue.hardware_concurrency();
        Self {
            queue,
            sources: HashMap::new(),
            accum_buffer: vec![Vec::new(); num_accum],
            next_src_id: 0,
        }
    }
}

impl<T: Generator + Send + Sync, Q: Queue> Panner<T> for EqualPowerPanner<T, Q> {
    type SourceId = SourceId;

    fn insert(&mut self, generator: T) -> SourceId {
        let new_next_id = self.next_src_id.checked_add(1).unwrap();
        let id = SourceId(self.next_src_id);

        self.sources.insert(
            id,
            Source {
                generator,
                buffer: Vec::new(),
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

impl<T: Generator + Send + Sync, Q: Queue> Generator for EqualPowerPanner<T, Q> {
    fn render(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        assert_eq!(to.len(), 2, "output must be stereo");
        let num_channels = to.len();
        let (to1, to2) = to.split_at_mut(1);
        let (to1, to2) = (&mut to1[0][range.clone()], &mut to2[0][range.clone()]);

        let num_samples = range.len();

        let mut sources: Vec<_> = self.sources
            .iter_mut()
            .filter_map(|(_, s)| if s.generator.is_active() {
                Some(s)
            } else {
                s.direction.update_multi(num_samples as f64);
                s.generator.skip(num_samples);
                None
            })
            .collect();

        if sources.len() > self.accum_buffer.len() * 2 && self.accum_buffer.len() > 1 {
            // Two-phase reduction
            // Source buffer -> Accumulation buffer -> Output buffer
            self.queue.foreach(&mut sources, |_, source| {
                assert_eq!(num_channels, 2);
                source.buffer.resize(cmp::max(1, num_channels), Vec::new());

                let gain1 = source.channel_gain();
                source.direction.update_multi(num_samples as f64);
                let gain2 = source.channel_gain();

                let (buf1, buf2) = source.buffer.split_at_mut(1);
                let (buf1, buf2) = (&mut buf1[0], &mut buf2[0]);

                buf1.resize(num_samples, 0.0);
                buf2.resize(num_samples, 0.0);

                source.generator.render(&mut [buf1], 0..num_samples);

                let delta_gain = [
                    (gain2[0] - gain1[0]) / num_samples as f32,
                    (gain2[1] - gain1[1]) / num_samples as f32,
                ];

                let mut gain = gain1.clone();
                let _ = (&buf1[0..num_samples], &buf2[0..num_samples]);
                for i in 0..num_samples {
                    let x = buf1[i];
                    buf1[i] = x * gain[0];
                    buf2[i] = x * gain[1];

                    gain[0] += delta_gain[0];
                    gain[1] += delta_gain[1];
                }
            });

            let num_accum_buffers = self.accum_buffer.len();
            self.queue.foreach(
                &mut self.accum_buffer,
                |i, accum_buffer| {
                    accum_buffer.resize(num_channels, Vec::new());

                    let start_src = sources.len() * i / num_accum_buffers;
                    let end_src = sources.len() * (i + 1) / num_accum_buffers;

                    for (ch, ch_ab) in accum_buffer.iter_mut().enumerate() {
                        ch_ab.clone_from(&sources[start_src].buffer[ch]);
                        for src_i in start_src + 1..end_src {
                            let ref src = sources[src_i].buffer[ch][0..ch_ab.len()];
                            for i in 0..ch_ab.len() {
                                ch_ab[i] += src[i];
                            }
                        }
                    }
                },
            );

            to1.copy_from_slice(&self.accum_buffer[0][0]);
            to2.copy_from_slice(&self.accum_buffer[0][1]);

            for ab_i in 1..self.accum_buffer.len() {
                let ref accum_buffer = self.accum_buffer[ab_i];
                for i in 0..num_samples {
                    to1[i] += accum_buffer[0][i];
                    to2[i] += accum_buffer[1][i];
                }
            }

        // TODO: add one-phase reduction path
        } else {
            // Serialized
            // 1x accumulation buffer -> Output buffer
            let ref mut accum_buffer = self.accum_buffer[0];
            accum_buffer.resize(num_channels, Vec::new());
            let ref mut tmp_buffer = accum_buffer[0];
            tmp_buffer.resize(num_samples, 0.0);

            for (i, source) in sources.iter_mut().enumerate() {
                source.generator.render(&mut [tmp_buffer], 0..num_samples);

                let gain1 = source.channel_gain();
                source.direction.update_multi(num_samples as f64);
                let gain2 = source.channel_gain();

                let delta_gain = [
                    (gain2[0] - gain1[0]) / num_samples as f32,
                    (gain2[1] - gain1[1]) / num_samples as f32,
                ];

                let mut gain = gain1.clone();

                if i == 0 {
                    for i in 0..num_samples {
                        let x = tmp_buffer[i];
                        to1[i] = x * gain[0];
                        to2[i] = x * gain[1];

                        gain[0] += delta_gain[0];
                        gain[1] += delta_gain[1];
                    }
                } else {
                    for i in 0..num_samples {
                        let x = tmp_buffer[i];
                        to1[i] += x * gain[0];
                        to2[i] += x * gain[1];

                        gain[0] += delta_gain[0];
                        gain[1] += delta_gain[1];
                    }
                }
            }

            if sources.len() == 0 {
                for i in 0..num_samples {
                    to1[i] = 0.0;
                    to2[i] = 0.0;
                }
            }
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
        false
    }
}
