//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/// A simple artificial reverberation filter based on Feedback Delay Networks,
/// which is particularly suitable to simulate a late reverberation, but not
/// early reflections.
///
/// This implementation features:
///
///  - 8 feedback lines and the Hadamard feedback matrix.
///  - An integrated first-order low-pass delay-line damping filter, which
///    causes high-frequency component to decay faster. It does not include a
///    room filter, though.
///  - Support for up to 8 output channels.
///
/// This filter accepts a single channel input signal and up to 8 output
/// channels.
use primal::is_prime;
use arrayvec::ArrayVec;
use std::ops::Range;

use ysr2_common::slicezip::{SliceZipMut, IndexByVal, IndexByValMut};
use Filter;

const WIDTH: usize = 8;

#[derive(Debug, Clone)]
pub struct MatrixReverb {
    delay_lines: [DelayLine; WIDTH],

    // Loop filter
    // y[n] = c0 x[n] + c1 y[n - 1]
    lpf_states: [f32; WIDTH],
    lpf_coef0: f32,
    lpf_coef1: f32,

    tail_len: f64,
    tail_remaining: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct MatrixReverbParams {
    /// RT60 reverbration time, measured in samples.
    pub reverb_time: f64,

    /// The mean delay-line length, measured in samples.
    pub mean_delay_time: f64,

    /// The amount of variation in delay line lengths, specified in the range
    /// `[0, 1]`.
    pub diffusion: f64,

    /// RT60 reverbration time for high frequency input (the reference frequency
    /// is specified by `high_frequency_ref`), relative to `reverb_time`.
    ///
    /// Must be in the range `[0, 1]`.
    pub reverb_time_hf_ratio: f64,

    /// The reference frequency at the properties of high frequency effects are
    /// specified. The value is relative to the sampling frequency and must be
    /// specified in the range `[0, 0.5]`.
    pub high_frequency_ref: f64,
}

#[derive(Debug, Clone)]
struct DelayLine {
    buffer: Vec<f32>,
    position: usize,
}

/// A reference to `DelayLine`.
///
/// Using `DelayLineRef` in place of a mutable reference of `[DelayLine; WIDTH]`
/// showed a 20% improvement in the performance because the code gen could store
/// `position` in a register.
struct DelayLineRef<'a> {
    buffer: &'a mut [f32],
    position_ref: &'a mut usize,
    position: usize,
}

impl MatrixReverb {
    pub fn new(params: &MatrixReverbParams) -> Self {
        // Decide the delay line lengths
        let mut lens = (0..WIDTH)
            .map(|i| {
                let factor = 2.0f64.powf(
                    (i as f64 - (WIDTH - 1) as f64 / 2.0) * (1.0 / WIDTH as f64) *
                        params.diffusion,
                );
                let f_len = params.mean_delay_time * factor;
                (f_len.max(2.0)) as usize | 1
            })
            .collect::<ArrayVec<[_; WIDTH]>>()
            .into_inner()
            .unwrap();

        // Choose prime numbers
        for i in 0..WIDTH {
            let mut start = lens[i];
            if i > 0 && lens[i - 1] >= start {
                start = lens[i - 1] + 2;
            }
            assert!(start % 2 == 1);

            let mut k = start;
            lens[i] = loop {
                if is_prime(k as u64) {
                    break k;
                }
                k += 2;
            };
        }

        // Create delay lines
        let delay_lines = lens.iter()
            .map(|x| DelayLine::new(*x))
            .collect::<ArrayVec<[_; WIDTH]>>()
            .into_inner()
            .unwrap();

        // Compute the actual mean delay line lengths
        let total_delay_time: usize = lens.iter().sum();
        let mean_delay_time = total_delay_time as f64 / lens.len() as f64;

        let dc_decay_time = params.reverb_time;
        let hf_decay_time = dc_decay_time * params.reverb_time_hf_ratio;

        let dc_loop_gain = 0.001f64.powf(mean_delay_time / dc_decay_time);
        let hf_loop_gain = 0.001f64.powf(mean_delay_time / hf_decay_time);

        // Design a one-pole LPF with the gain `lpf_gain` at the frequency
        // `params.high_frequency_ref`
        //
        //     Transfer function: H(z) = (1 - a) / (1 - a z^{-1})
        //         where a in [0, 1]
        //
        //     Gain:   G(omega) = (1 - a) / |1 - a cos(omega) + a sin(omega) i|
        //           G^2(omega) = (1 - a)^2 / ((1 - a cos)^2 + (a sin)^2)
        //                      = (1 - a)^2 / (1 - 2a cos + a^2)
        //                 G(0) = 1
        //
        // We solve this for `a`:
        //
        //         (1 - G^2 cos) - sqrt((1 - G^2 cos)^2 - (1-G^2)^2)
        //     a = -----------------------------------------------
        //                             1 - G^2
        //
        let lpf_gain = hf_loop_gain / dc_loop_gain;
        let lpf_coef = if lpf_gain < 0.9999 {
            let ref_freq = params.high_frequency_ref * (::std::f64::consts::PI * 2.0);
            let t1 = 1.0 - lpf_gain * lpf_gain;
            let t2 = 1.0 - lpf_gain * lpf_gain * ref_freq.cos();
            (t2 - (t2 * t2 - t1 * t1).sqrt()) / t1
        } else {
            0.0
        };

        let lpf_coef0 = (1.0 - lpf_coef) * dc_loop_gain;
        let lpf_coef1 = lpf_coef;

        // Compensate for the factor of `fwht8`
        let lpf_coef0 = lpf_coef0 / (WIDTH as f64).sqrt();

        Self {
            delay_lines,
            lpf_states: [0.0; WIDTH],
            lpf_coef0: lpf_coef0 as f32,
            lpf_coef1: lpf_coef1 as f32,
            tail_len: params.reverb_time * 4.0,
            tail_remaining: 0.0,
        }
    }
}

impl Filter for MatrixReverb {
    fn render(
        &mut self,
        to: &mut [&mut [f32]],
        range: Range<usize>,
        from: Option<(&[&[f32]], Range<usize>)>,
    ) {
        let from = from.map(|(inputs, from_range)| {
            assert_eq!(inputs.len(), 1, "Input must be a single channel signal");
            assert_eq!(from_range.len(), range.len());
            &inputs[0][from_range]
        });
        let mut lpf_states = self.lpf_states.clone();
        let lpf_coef0 = self.lpf_coef0;
        let lpf_coef1 = self.lpf_coef1;
        let mut delay_lines = self.delay_lines
            .iter_mut()
            .map(DelayLine::borrow)
            .collect::<ArrayVec<[_; WIDTH]>>()
            .into_inner()
            .ok()
            .unwrap();
        let mut max_input_abs = 0.0f32;

        macro_rules! case {
            ($len:expr) => ({
                let mut writer = SliceZipMut::<[f32; $len], _, _>::new(to, range.clone());
                for i in 0..writer.len() {
                    let mut output: [f32; $len] = writer.get(i).unwrap();
                    let input = if let Some(from) = from {
                        from[i]
                    } else {
                        output[0]
                    };
                    max_input_abs = max_input_abs.max(input.abs());

                    // Read delay lines
                    let mut delayed = delay_lines
                        .iter()
                        .map(DelayLineRef::peek)
                        .collect::<ArrayVec<[_; WIDTH]>>()
                        .into_inner()
                        .unwrap();

                    // Write output
                    output.copy_from_slice(&delayed[0..$len]);
                    writer.set(i, output);

                    // Feed input
                    delayed[7] += input;

                    // Apply diffusion matrix
                    // (Actually, the matrix `fwht8` represents have eigenvalues
                    //  greater than `1`. This is compensated by `lpf_coef0`)
                    fwht8(&mut delayed);

                    // Apply loop filter
                    for i in 0..WIDTH {
                        lpf_states[i] = delayed[i] * lpf_coef0 + lpf_states[i] * lpf_coef1;
                    }

                    // Feed delay lines
                    for i in 0..WIDTH {
                        delay_lines[i].feed(lpf_states[i]);
                    }
                }
            })
        }
        if from.is_some() {
            match to.len() {
                1 => case!(1),
                2 => case!(2),
                3 => case!(3),
                4 => case!(4),
                5 => case!(5),
                6 => case!(6),
                7 => case!(7),
                8 => case!(8),
                _ => panic!("too many channels"),
            }
        } else {
            assert_eq!(to.len(), 1, "Input must be a single channel signal");
            case!(1);
        }

        self.lpf_states = lpf_states;
        for x in delay_lines.iter_mut() {
            x.finalize();
        }

        self.tail_remaining =
            (self.tail_remaining - range.len() as f64).max(max_input_abs as f64 * self.tail_len);
    }

    /// Always return `Some(1)`.
    fn num_input_channels(&self) -> Option<usize> {
        Some(1)
    }

    /// Always return `None` since it accepts up to 8 output channels.
    fn num_output_channels(&self) -> Option<usize> {
        None
    }

    fn is_active(&self) -> bool {
        self.tail_remaining > 0.0
    }

    fn skip(&mut self, num_samples: usize) {
        let num_processed_samples = if num_samples as f64 >= self.tail_remaining {
            num_samples
        } else if self.tail_remaining >= 0.0 {
            self.tail_remaining as usize
        } else {
            0
        };

        // Note: Make sure to synchronize this with `render`
        let mut lpf_states = self.lpf_states.clone();
        let lpf_coef0 = self.lpf_coef0;
        let lpf_coef1 = self.lpf_coef1;
        let mut delay_lines = self.delay_lines
            .iter_mut()
            .map(DelayLine::borrow)
            .collect::<ArrayVec<[_; WIDTH]>>()
            .into_inner()
            .ok()
            .unwrap();

        for _ in 0..num_processed_samples {
            // Read delay lines
            let mut delayed = delay_lines
                .iter()
                .map(DelayLineRef::peek)
                .collect::<ArrayVec<[_; WIDTH]>>()
                .into_inner()
                .unwrap();

            // No input, no output

            // Apply diffusion matrix
            // (Actually, the matrix `fwht8` represents have eigenvalues
            //  greater than `1`. This is compensated by `lpf_coef0`)
            fwht8(&mut delayed);

            // Apply loop filter
            for i in 0..WIDTH {
                lpf_states[i] = delayed[i] * lpf_coef0 + lpf_states[i] * lpf_coef1;
            }

            // Feed delay lines
            for i in 0..WIDTH {
                delay_lines[i].feed(lpf_states[i]);
            }
        }

        self.lpf_states = lpf_states;
        for x in delay_lines.iter_mut() {
            x.finalize();
        }

        self.tail_remaining -= num_samples as f64;
    }

    fn reset(&mut self) {
        for x in self.lpf_states.iter_mut() {
            *x = 0.0;
        }
        for x in self.delay_lines.iter_mut() {
            x.reset();
        }
    }
}

impl DelayLine {
    fn new(len: usize) -> Self {
        DelayLine {
            buffer: vec![0.0; len],
            position: 0,
        }
    }

    fn reset(&mut self) {
        for x in self.buffer.iter_mut() {
            *x = 0.0;
        }
    }

    fn borrow(&mut self) -> DelayLineRef {
        assert!(self.position < self.buffer.len());

        DelayLineRef {
            position: self.position,
            position_ref: &mut self.position,
            buffer: &mut self.buffer[..],
        }
    }
}

impl<'a> DelayLineRef<'a> {
    fn finalize(&mut self) {
        *self.position_ref = self.position;
    }

    fn peek(&self) -> f32 {
        unsafe { *self.buffer.get_unchecked(self.position) }
    }

    fn feed(&mut self, x: f32) {
        unsafe {
            *self.buffer.get_unchecked_mut(self.position) = x;
        }
        self.position += 1;
        if self.position >= self.buffer.len() {
            self.position = 0;
        }
    }
}

/// Apply the fast Walsh-Hadamard transform.
fn fwht8(x: &mut [f32; 8]) {
    let mut butterfly = |x1, x2| {
        let a1 = x[x1];
        let a2 = x[x2];
        x[x1] = a1 + a2;
        x[x2] = a1 - a2;
    };

    butterfly(0, 4);
    butterfly(1, 5);
    butterfly(2, 6);
    butterfly(3, 7);

    butterfly(0, 2);
    butterfly(1, 3);
    butterfly(4, 6);
    butterfly(5, 7);

    butterfly(0, 1);
    butterfly(2, 3);
    butterfly(4, 5);
    butterfly(6, 7);
}
