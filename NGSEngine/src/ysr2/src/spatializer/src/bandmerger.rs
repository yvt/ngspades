//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Merges multi-band audio samples (e.g., `[FdQuant<[f32; 8]>]` to `[f32]`).
use std::marker::PhantomData;
use ysr2_filters::biquad::{BiquadKernelState, BiquadCoefs, eq};
use {FdQuant, BaseFdQuant, BaseNum, Float};

/// Merges multi-band audio samples.
pub trait BandMerger {
    type Quantity: BaseFdQuant;

    /// Generate a single channel audio stream from given multi-band audio
    /// streams each of which describes a part of the frequency spectrum.
    ///
    /// `output.len()` must be equal to `input.len()`.
    fn merge(
        &mut self,
        output: &mut [<Self::Quantity as BaseFdQuant>::Scalar],
        input: &[Self::Quantity],
    );

    /// Reset the internal state of the internal crossover network.
    fn reset(&mut self);
}

/// `BandMerger` based on a fourth-order Linkwitz-Riley crossover filters (LR-4).
///
/// LR-4 crossover filters are constructed by cascading two second-order
/// Butterworth low-pass/high-pass filters.
///
/// Since it is based on crossover filters, it has the unity gain property i.e.
/// if you supply it with the same audio signal for all bands, it will behave as
/// an all-pass filter.
pub struct Lr4BandMerger<T> {
    coefs: Vec<BiquadCoefs>,
    states: Vec<BiquadKernelState>,
    _phantom: PhantomData<T>,
}

impl<T> Lr4BandMerger<T> {
    fn reset_inner(&mut self) {
        for state in self.states.iter_mut() {
            state.reset();
        }
    }
}

impl Lr4BandMerger<FdQuant<[f32; 8]>> {
    /// Construct a `Lr4BandMerger` with given crossover frequencies (normalized).
    ///
    /// `crossover_freqs.len()` must be 7.
    pub fn new(crossover_freqs: &[f64]) -> Self {
        assert_eq!(crossover_freqs.len(), 7);

        use std::f64::consts::FRAC_1_SQRT_2;
        let mut coefs = Vec::with_capacity(8 * 3);
        for i in 0..8 {
            let fqs = [
                crossover_freqs[((i & 0b000) | 4) - 1],
                crossover_freqs[((i & 0b100) | 2) - 1],
                crossover_freqs[((i & 0b110) | 1) - 1],
            ];
            coefs.push(if (i & 4) != 0 {
                eq::high_pass_filter(fqs[0], FRAC_1_SQRT_2)
            } else {
                eq::low_pass_filter(fqs[0], FRAC_1_SQRT_2)
            });
            coefs.push(if (i & 2) != 0 {
                eq::high_pass_filter(fqs[1], FRAC_1_SQRT_2)
            } else {
                eq::low_pass_filter(fqs[1], FRAC_1_SQRT_2)
            });
            coefs.push(if (i & 2) != 0 {
                eq::high_pass_filter(fqs[2], FRAC_1_SQRT_2)
            } else {
                eq::low_pass_filter(fqs[2], FRAC_1_SQRT_2)
            });
        }

        let states = vec![BiquadKernelState::new(); coefs.len() * 2];

        Self {
            coefs,
            states,
            _phantom: PhantomData,
        }
    }
}

impl<T> BandMerger for Lr4BandMerger<FdQuant<[T; 8]>>
where
    T: BaseNum + Float,
{
    type Quantity = FdQuant<[T; 8]>;

    fn merge(&mut self, output: &mut [T], input: &[FdQuant<[T; 8]>]) {
        assert_eq!(output.len(), input.len());

        let ref coefs = self.coefs[0..8 * 3];
        let ref mut states = self.states[0..8 * 3 * 2];

        for (y, x) in output.iter_mut().zip(input.iter()) {
            let bands = *x.get_ref();

            // Apply filters
            let mut output = 0.0;
            for i in 0..8 {
                let mut band = bands[i].to_f64().unwrap();
                for k in 0..3 {
                    let index = i * 3 + k;
                    let ref coef = coefs[index];
                    band = states[index * 2].apply_to_sample(band, coef);
                    band = states[index * 2 + 1].apply_to_sample(band, coef);
                }
                output += band;
            }

            *y = T::from(output).unwrap();
        }
    }

    fn reset(&mut self) {
        self.reset_inner();
    }
}
