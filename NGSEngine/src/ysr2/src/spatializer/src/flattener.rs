//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Generates audio samples from a series of directional impulses.
use cgmath::{Vector3, Zero};
use {BaseNum, BaseFdQuant};

/// Flattens a series of directional impulses (triples of time, amplitude, and
/// direction) and generates audio samples.
pub trait Flattener {
    type Quantity: BaseFdQuant;

    /// Record a directional impulse.
    ///
    /// `direction` does not have to be normalized. It must not be a zero vector.
    fn record_imp_dir(&mut self, time: f32, amplitude: Self::Quantity, direction: Vector3<f32>);
}

/// Generates a sampled series of the band-limited impulse (BLI) with a
/// fractional position.
pub trait BliSource<S> {
    type Output: AsRef<[S]>;

    /// Generates a sampled series of the band-limited impulse (BLI) with a
    /// fractional position `frac_pos`.
    ///
    /// `frac_pos` must be in the range `[0, 1)`.
    fn bli(&self, frac_pos: f32) -> Self::Output;
}

impl<'a, T: BliSource<S>, S> BliSource<S> for &'a T {
    type Output = T::Output;

    fn bli(&self, frac_pos: f32) -> Self::Output {
        (*self).bli(frac_pos)
    }
}

/// Rectangular windowed `BliSource`.
pub struct RectBliSource;

impl<T: BaseNum> BliSource<T> for RectBliSource {
    type Output = [T; 1];

    fn bli(&self, _frac_pos: f32) -> Self::Output {
        [T::one()]
    }
}

/// Lanczos (a = 4) windowed `BliSource` with 4-bit fractional precision.
///
/// Has an intrinsic latency of 3 samples.
///
/// # Examples
///
///     # use ysr2_spatializer::flattener::{BliSource, Lanczos4BliSource};
///     let source = Lanczos4BliSource::<f32>::new();
///
///     // Generate an impulse at the position 3
///     debug_assert_eq!(source.bli(0.0), [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
///
pub struct Lanczos4BliSource<T>([[T; 8]; 16]);

impl<T: BaseNum> Lanczos4BliSource<T> {
    /// Construct a `Lanczos4BliSource`.
    ///
    /// This might take a while to generate a lookup table. It is advised to
    /// call this only once in the initialization phase (for example by using
    /// `lazy_static`) if possible.
    pub fn new() -> Self {
        fn sinc(x: f64) -> f64 {
            use std::f64::consts::PI;
            (x * PI).sin() / (x * PI)
        }
        let mut table = [[T::zero(); 8]; 16];
        table[0][3] = T::one();
        for off in 1..16 {
            let mut pos: f64 = -3.0 - off as f64 / 16.0;
            for i in 0..8 {
                table[off][i] = T::from(sinc(pos) * sinc(pos / 4.0)).unwrap();
                pos += 1.0;
            }
        }
        Lanczos4BliSource(table)
    }
}

impl<T: BaseNum> BliSource<T> for Lanczos4BliSource<T> {
    type Output = [T; 8];

    fn bli(&self, frac_pos: f32) -> Self::Output {
        let i = (frac_pos * 16.0) as usize;
        debug_assert!(i < 16);
        self.0[i]
    }
}

/// Computes a contribution to each audio channel from a given incoming direction.
pub trait McChannelMapper<S> {
    /// Get the number of output audio channels.
    fn num_channels(&self) -> usize;

    /// Compute a contribution to each audio channel from a given incoming direction.
    ///
    /// The result is stored into `out`. From a perspective of energy preservation,
    /// the sum of all outputs should be less than or equal to 1, but it is not
    /// a hard limit.
    fn map(&self, direction: Vector3<f32>, out: &mut [S]);
}

/// `McChannelMapper` with a single output channel.
pub struct MonoMcChannelMapper;

impl<S: BaseNum> McChannelMapper<S> for MonoMcChannelMapper {
    fn num_channels(&self) -> usize {
        1
    }

    fn map(&self, _direction: Vector3<f32>, out: &mut [S]) {
        for x in out.iter_mut() {
            *x = S::one();
        }
    }
}

/// `Flattener` that uses `McChannelMapper` to derive contributions to output
/// channels for a given direction and generates multi-channel audio data.
///
/// # Examples
///
///     # use ysr2_spatializer::flattener::*;
///     # use ysr2_spatializer::cgmath::Vector3;
///     let mut flt = McFlattener::new(RectBliSource, MonoMcChannelMapper, 64);
///
///     let time = 8.0f32;
///     let amplitude = 2.0f32; // f32: BaseFdQuant
///     flt.record_imp_dir(time, amplitude, Vector3::unit_z());
///
///     let samples = flt.get_channel_samples(0).unwrap();
///     assert_eq!(samples[8], 2.0f32);
///
pub struct McFlattener<Q: BaseFdQuant, B, M> {
    bli: B,
    mapper: M,
    channels: Vec<Vec<Q>>,
    contrib: Vec<Q::Scalar>,
}

impl<Q, B, M> McFlattener<Q, B, M>
where
    Q: BaseFdQuant,
    B: BliSource<Q::Scalar>,
    M: McChannelMapper<Q::Scalar>,
{
    pub fn new(bli: B, mapper: M, num_samples: usize) -> Self {
        McFlattener {
            bli,
            channels: (0..mapper.num_channels())
                .map(|_| vec![Default::default(); num_samples])
                .collect(),
            contrib: vec![Zero::zero(); mapper.num_channels()],
            mapper,
        }
    }

    pub fn clear(&mut self) {
        for channel in self.channels.iter_mut() {
            for x in channel.iter_mut() {
                *x = Default::default();
            }
        }
    }

    pub fn get_channel_samples(&self, channel_index: usize) -> Option<&[Q]> {
        self.channels.get(channel_index).map(|v| v.as_slice())
    }
}

impl<Q, B, M> Flattener for McFlattener<Q, B, M>
where
    Q: BaseFdQuant,
    B: BliSource<Q::Scalar>,
    M: McChannelMapper<Q::Scalar>,
{
    type Quantity = Q;
    fn record_imp_dir(&mut self, time: f32, amplitude: Self::Quantity, direction: Vector3<f32>) {
        debug_assert!(time >= 0.0);

        let time_i = time as usize;
        let time_f = time - time_i as f32;

        let contribs = self.contrib.as_mut_slice();
        self.mapper.map(direction, contribs);

        let bli = self.bli.bli(time_f);
        let bli_smp = bli.as_ref();

        for (channel, &contrib) in self.channels.iter_mut().zip(contribs.iter()) {
            if time_i + bli_smp.len() > channel.len() || contrib.is_zero() {
                continue;
            }

            for (&x, y) in bli_smp.iter().zip(channel[time_i..].iter_mut()) {
                *y += amplitude * (x * contrib);
            }
        }
    }
}
