//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;

use super::BiquadCoefs;
use siso::SisoFilter;
use utils::apply_by_sample;

#[derive(Debug, Clone)]
pub struct SimpleBiquadKernel {
    coefs: BiquadCoefs,
    states: Vec<(f64, f64)>,
}

impl SimpleBiquadKernel {
    pub fn new(coefs: &BiquadCoefs, num_channels: usize) -> Self {
        Self {
            coefs: coefs.clone(),
            states: vec![(0.0, 0.0); num_channels],
        }
    }
}

impl SisoFilter for SimpleBiquadKernel {
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
            let coefs = self.coefs.clone();
            let ref mut state = self.states[i];
            apply_by_sample(
                &mut to[i][range.clone()],
                from.as_ref().map(|&(ref inputs, ref in_range)| &inputs[i][in_range.clone()]),
                move |iter| {
                    let mut st = *state;
                    for x in iter {
                        // Direct form 2 implementation
                        let t = *x as f64 - st.0 * coefs.a1 - st.1 * coefs.a2;
                        *x = (t * coefs.b0 + st.0 * coefs.b1 + st.1 * coefs.b2) as f32;
                        st = (t, st.0);
                    }
                    *state = st;
                },
            );
        }
    }

    fn is_active(&self) -> bool {
        for x in self.states.iter() {
            if x.0.abs().max(x.1.abs()) > 1.0e-10 {
                return true;
            }
        }
        false
    }

    fn skip(&mut self, num_samples: usize) {
        // FIXME: there should be a O(1) method for this
        let ref coefs = self.coefs;
        for state in self.states.iter_mut() {
            for _ in 0..num_samples {
                let t = -state.0 * coefs.a1 - state.1 * coefs.a2;
                *state = (t, state.0);
            }
        }
    }

    fn reset(&mut self) {
        for x in self.states.iter_mut() {
            *x = (0.0, 0.0);
        }
    }
}
