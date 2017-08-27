//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines a trait for a single-input and single-output causal filter.
use std::ops::Range;

/// A single-input and single-output causal filter.
pub trait SisoFilter {
    /// Apply the filter to the input signal `from.unwrap_or((to, range))` and
    /// write the output to `to`.
    ///
    /// If `from` is `Some((inputs, from_range))`, `from_range.len()` must be
    /// equal to `range.len()` and `inputs.len()` must be equal to `to.len()`.
    fn render(
        &mut self,
        to: &mut [&mut [f32]],
        range: Range<usize>,
        from: Option<(&[&[f32]], Range<usize>)>,
    );

    /// Apply the filter to the signal `to` in-place.
    fn render_inplace(&mut self, to: &mut [&mut [f32]], range: Range<usize>) {
        self.render(to, range, None)
    }

    /// Determine whether a following call to `render` generates a non-zero
    /// (more precisely, above a predetermined threshold) signal even with a
    /// zero input signal.
    fn is_active(&self) -> bool;

    /// Feed `num_samples` samples with zero values and discard the output.
    fn skip(&mut self, num_samples: usize);

    /// Reset the filter to the initial state.
    fn reset(&mut self);
}
