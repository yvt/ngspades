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
    /// `crossover_freqs.len()` must be 7 and in an ascending order.
    pub fn new(crossover_freqs: &[f64]) -> Self {
        assert_eq!(crossover_freqs.len(), 7);

        use std::f64::consts::FRAC_1_SQRT_2;
        let mut coefs = Vec::with_capacity(14);
        for i in 0..8 {
            let fq = crossover_freqs[(i | 1) - 1];
            coefs.push(if (i & 1) != 0 {
                eq::high_pass_filter(fq, FRAC_1_SQRT_2)
            } else {
                eq::low_pass_filter(fq, FRAC_1_SQRT_2)
            });
        }
        for i in 0..4 {
            let fq = crossover_freqs[((i * 2) | 2) - 1];
            coefs.push(if (i & 1) != 0 {
                eq::high_pass_filter(fq, FRAC_1_SQRT_2)
            } else {
                eq::low_pass_filter(fq, FRAC_1_SQRT_2)
            });
        }
        for i in 0..2 {
            let fq = crossover_freqs[((i * 4) | 4) - 1];
            coefs.push(if (i & 1) != 0 {
                eq::high_pass_filter(fq, FRAC_1_SQRT_2)
            } else {
                eq::low_pass_filter(fq, FRAC_1_SQRT_2)
            });
        }

        let states = vec![BiquadKernelState::new(); 28 + 16 + 24];

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

        let ref coefs = self.coefs[0..14];
        let ref mut states = self.states[0..68];

        for (y, x) in output.iter_mut().zip(input.iter()) {
            let bands = *x.get_ref();

            let mut tmp = [0f64; 8];

            for i in 0..8 {
                let mut x = bands[i].to_f64().unwrap();

                let ref coef = coefs[i];

                // Two second-order Butterworth LPF/HPF
                let index = i * 2;
                x = states[index].apply_to_sample(x, coef);
                x = states[index + 1].apply_to_sample(x, coef);

                tmp[i] = x;
            }

            for i in 0..4 {
                let x = tmp[i * 2] + tmp[i * 2 + 1];

                // All-pass filters
                let mut x = {
                    let index = 28 + i * 4;
                    let ref coef1 = coefs[(i * 2) ^ 2];
                    let ref coef2 = coefs[(i * 2 + 1) ^ 2];
                    let mut x1 = states[index].apply_to_sample(x, coef1);
                    let mut x2 = states[index + 1].apply_to_sample(x, coef2);
                    x1 = states[index + 2].apply_to_sample(x1, coef1);
                    x2 = states[index + 3].apply_to_sample(x2, coef2);
                    x1 + x2
                };

                // Two second-order Butterworth LPF/HPF
                let ref coef = coefs[i + 8];
                let index = (i + 8) * 2;
                x = states[index].apply_to_sample(x, coef);
                x = states[index + 1].apply_to_sample(x, coef);

                tmp[i * 2] = x;
            }

            for i in 0..2 {
                let x = tmp[i * 4] + tmp[i * 4 + 2];

                // All-pass filters
                let x = {
                    let index = 44 + i * 4;
                    let ref coef1 = coefs[(i * 4) ^ 4];
                    let ref coef2 = coefs[(i * 4 + 1) ^ 4];
                    let mut x1 = states[index].apply_to_sample(x, coef1);
                    let mut x2 = states[index + 1].apply_to_sample(x, coef2);
                    x1 = states[index + 2].apply_to_sample(x1, coef1);
                    x2 = states[index + 3].apply_to_sample(x2, coef2);
                    x1 + x2
                };

                let x = {
                    let index = 44 + 8 + i * 4;
                    let ref coef1 = coefs[(i * 4 + 2) ^ 4];
                    let ref coef2 = coefs[(i * 4 + 3) ^ 4];
                    let mut x1 = states[index].apply_to_sample(x, coef1);
                    let mut x2 = states[index + 1].apply_to_sample(x, coef2);
                    x1 = states[index + 2].apply_to_sample(x1, coef1);
                    x2 = states[index + 3].apply_to_sample(x2, coef2);
                    x1 + x2
                };

                let mut x = {
                    let index = 44 + 8 * 2 + i * 4;
                    let ref coef1 = coefs[(i * 2 + 8) ^ 2];
                    let ref coef2 = coefs[(i * 2 + 9) ^ 2];
                    let mut x1 = states[index].apply_to_sample(x, coef1);
                    let mut x2 = states[index + 1].apply_to_sample(x, coef2);
                    x1 = states[index + 2].apply_to_sample(x1, coef1);
                    x2 = states[index + 3].apply_to_sample(x2, coef2);
                    x1 + x2
                };

                // Two second-order Butterworth LPF/HPF
                let ref coef = coefs[i + 12];
                let index = (i + 12) * 2;
                x = states[index].apply_to_sample(x, coef);
                x = states[index + 1].apply_to_sample(x, coef);

                tmp[i * 4] = x;
            }

            let output = tmp[0] + tmp[4];

            *y = T::from(output).unwrap();
        }
    }

    fn reset(&mut self) {
        self.reset_inner();
    }
}
