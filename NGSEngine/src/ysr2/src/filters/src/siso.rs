//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines a trait for a single-input and single-output causal filter.
use std::ops::Range;
use std::borrow::BorrowMut;
use Filter;

/// A single-input and single-output causal filter.
///
/// Even though it is a single-input and single-output filter, it can accept
/// multi-channel signals. In this case, the same filter should be applied to
/// each channel. This also means the number of input channels and that of
/// output channel must be equal.
pub trait SisoFilter: Filter {
    /// Return the number of channels of the input/output signal.
    ///
    /// `None` is a wild card value that means it can accept any number of
    /// channels. Even in this case, the number of input channels and that of
    /// output channel must be equal.
    ///
    /// The returned value must be equal to both of `num_input_channels()` and
    /// `num_output_channels()`.
    fn num_channels(&self) -> Option<usize> {
        let inp = self.num_input_channels();
        let out = self.num_output_channels();
        assert_eq!(inp, out);
        inp
    }
}

/// SISO filter that applies multiple `SisoFilter`s in a serial fashion.
pub struct CascadedSisoFilter<T>(Vec<T>);

impl<T> CascadedSisoFilter<T>
where
    T: BorrowMut<SisoFilter>,
{
    /// Construct a `CascadedSisoFilter`.
    ///
    /// The number of channels of every element of `filters` must match.
    pub fn new(filters: Vec<T>) -> Self {
        if filters.len() > 0 {
            let mut num_channels = None;
            for filter in filters.iter() {
                let filter = filter.borrow();
                let flt_num_channels = filter.num_channels();
                num_channels = num_channels.or(flt_num_channels);
                assert_eq!(num_channels, flt_num_channels.or(num_channels));
            }
        }

        CascadedSisoFilter(filters)
    }
}

impl<T> SisoFilter for CascadedSisoFilter<T>
where
    T: BorrowMut<SisoFilter>,
{
    fn num_channels(&self) -> Option<usize> {
        self.0
            .iter()
            .filter_map(|f| f.borrow().num_channels())
            .nth(0)
    }
}

impl<T> Filter for CascadedSisoFilter<T>
where
    T: BorrowMut<SisoFilter>,
{
    fn render(
        &mut self,
        to: &mut [&mut [f32]],
        range: Range<usize>,
        mut from: Option<(&[&[f32]], Range<usize>)>,
    ) {
        for filter in self.0.iter_mut() {
            filter.borrow_mut().render(to, range.clone(), from.take());
        }
    }

    fn is_active(&self) -> bool {
        self.0.iter().any(|f| f.borrow().is_active())
    }

    fn num_input_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn num_output_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn skip(&mut self, num_samples: usize) {
        let num_channels = self.num_channels().unwrap();
        let mut iter = self.0.iter_mut();
        let mut buffer: Vec<Vec<f32>> = Vec::new();

        let mut cur = iter.next();

        // Try to use fast path as far as we can go
        while let Some(filter) = cur.take() {
            if filter.borrow().is_active() {
                // Can't use `skip`; we might have to take the slow path
                cur = Some(filter);
                break;
            }

            let filter = filter.borrow_mut();
            filter.skip(num_samples);

            cur = iter.next();
        }

        // If there are active filters in the middle, we need a temporary buffer
        while let Some(filter) = cur.take() {
            let filter = filter.borrow_mut();
            if iter.len() == 0 && buffer.len() == 0 {
                // If this is the last one then we don't care it's output
                filter.skip(num_samples);
            } else {
                if buffer.len() != num_channels {
                    // Allocate a temporary buffer
                    buffer = vec![vec![0.0; num_samples]; num_channels];
                }

                let mut buffer_refs: Vec<_> = buffer.iter_mut().map(Vec::as_mut_slice).collect();
                filter.render_inplace(buffer_refs.as_mut_slice(), 0..num_samples);
            }

            cur = iter.next();
        }
    }

    fn reset(&mut self) {
        for x in self.0.iter_mut() {
            x.borrow_mut().reset();
        }
    }
}
