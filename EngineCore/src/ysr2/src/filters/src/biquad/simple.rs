//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;

use Filter;
use super::BiquadCoefs;
use siso::SisoFilter;
use utils::apply_by_sample;

#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadKernelState(f64, f64);

impl BiquadKernelState {
    pub fn new() -> Self {
        BiquadKernelState(0.0, 0.0)
    }

    pub fn reset(&mut self) {
        self.0 = 0.0;
        self.1 = 0.0;
    }

    pub fn apply_to_sample(&mut self, x: f64, coefs: &BiquadCoefs) -> f64 {
        // Direct form 2 implementation
        let t = x - self.0.mul_add(coefs.a1, self.1 * coefs.a2);
        let y = t.mul_add(coefs.b0, self.0.mul_add(coefs.b1, self.1 * coefs.b2));
        self.1 = self.0;
        self.0 = t;
        y
    }

    pub fn is_active(&self) -> bool {
        self.0.abs().max(self.1.abs()) > 1.0e-10
    }

    pub fn skip(&mut self, num_samples: usize, coefs: &BiquadCoefs) {
        // FIXME: there should be a O(1) method for this
        for _ in 0..num_samples {
            self.apply_to_sample(0.0, coefs);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimpleBiquadKernel {
    coefs: BiquadCoefs,
    states: Vec<BiquadKernelState>,
}

impl SimpleBiquadKernel {
    pub fn new(coefs: &BiquadCoefs, num_channels: usize) -> Self {
        Self {
            coefs: coefs.clone(),
            states: vec![BiquadKernelState::new(); num_channels],
        }
    }
}

impl SisoFilter for SimpleBiquadKernel {
    fn num_channels(&self) -> Option<usize> {
        Some(self.states.len())
    }
}

impl Filter for SimpleBiquadKernel {
    fn render(
        &mut self,
        to: &mut [&mut [f32]],
        range: Range<usize>,
        from: Option<(&[&[f32]], Range<usize>)>,
    ) {
        // validate the range
        assert!(range.start <= range.end);
        for ch in to.iter() {
            let _ = &ch[range.clone()];
        }
        assert_eq!(self.states.len(), to.len());

        for i in 0..to.len() {
            let ref mut state = self.states[i];
            let ref coefs = self.coefs;
            apply_by_sample(
                &mut to[i][range.clone()],
                from.as_ref().map(|&(ref inputs, ref in_range)| &inputs[i][in_range.clone()]),
                move |iter| {
                    let mut st = *state;
                    let coefs = coefs.clone();
                    for x in iter {
                        *x = st.apply_to_sample(*x as f64, &coefs) as f32;
                    }
                    *state = st;
                },
            );
        }
    }

    fn is_active(&self) -> bool {
        self.states.iter().any(BiquadKernelState::is_active)
    }

    fn num_input_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn num_output_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn skip(&mut self, num_samples: usize) {
        for x in self.states.iter_mut() {
            x.skip(num_samples, &self.coefs);
        }
    }

    fn reset(&mut self) {
        for x in self.states.iter_mut() {
            x.reset();
        }
    }
}
