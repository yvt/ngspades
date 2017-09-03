//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![cfg_attr(test, feature(test))]
extern crate ysr2_common;

use std::ops::Range;

pub mod biquad;
pub mod siso;
mod utils;

/// A causal filter.
pub trait Filter {
    /// Apply the filter to the input signal `from.unwrap_or((to, range))` and
    /// write the output to `to`.
    ///
    /// - If `from` is `Some((inputs, from_range))`, `from_range.len()` must be
    ///   equal to `range.len()` and `inputs.len()` must be equal to `to.len()`.
    /// - If `num_output_channels()` is `Some(x)`, then `to.len()` must be equal
    ///   to `x`.
    /// - If `num_input_channels()` is `Some(x)`, then
    ///   `from.unwrap_or((to, range)).0.len()` must be equal to `x`.
    ///
    fn render(
        &mut self,
        to: &mut [&mut [f32]],
        range: Range<usize>,
        from: Option<(&[&[f32]], Range<usize>)>,
    );

    /// Apply the filter to the signal `to` in-place. Can be used only if
    /// `num_input_channels() == num_output_channels()`.
    ///
    /// This can be used a syntax sugar of `render(to, range, None)`.
    fn render_inplace(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        self.render(to, range, None)
    }

    /// Return the number of channels of the input signal.
    ///
    /// `None` indicates the value is not restricted, or governed by some
    /// other restrictions.
    fn num_input_channels(&self) -> Option<usize>;

    /// Return the number of channels of the output signal.
    ///
    /// `None` indicates the value is not restricted, or governed by some
    /// other restrictions.
    fn num_output_channels(&self) -> Option<usize>;

    /// Determine whether a following call to `render` generates a non-zero
    /// (more precisely, above a predetermined threshold) signal even with a
    /// zero input signal.
    fn is_active(&self) -> bool;

    /// Feed `num_samples` samples with zero values and discard the output.
    fn skip(&mut self, num_samples: usize);

    /// Reset the filter to the initial state.
    fn reset(&mut self);
}
