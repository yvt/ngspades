//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines a trait for a single-input and single-output causal filter.
use std::ops::Range;
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

/// SISO filter that outputs a signal identical to the input.
#[derive(Debug, Clone, Copy)]
pub struct IdentityFilter;

impl Filter for IdentityFilter {
    fn render(
        &mut self,
        to: &mut [&mut [f32]],
        range: Range<usize>,
        from: Option<(&[&[f32]], Range<usize>)>,
    ) {
        if let Some((input, ref in_range)) = from {
            assert_eq!(range.len(), in_range.len());
            assert_eq!(input.len(), to.len());
            for (to, from) in to.iter_mut().zip(input.iter()) {
                to[range.clone()].copy_from_slice(&from[in_range.clone()]);
            }
        }
    }

    fn is_active(&self) -> bool {
        false
    }

    fn num_input_channels(&self) -> Option<usize> {
        None
    }

    fn num_output_channels(&self) -> Option<usize> {
        None
    }

    fn skip(&mut self, _: usize) {}

    fn reset(&mut self) {}
}

impl SisoFilter for IdentityFilter {
    fn num_channels(&self) -> Option<usize> {
        None
    }
}

/// A series of zero or more `SisoFilter`s to be consumed by a
/// `CascadedSisoFilter`.
pub trait SisoFilters {
    fn for_each<F: FnMut(&SisoFilter)>(&self, f: F);
    fn for_each_mut<F: FnMut(&mut SisoFilter)>(&mut self, f: F);
}

impl<T: AsRef<SisoFilter> + AsMut<SisoFilter>> SisoFilters for Vec<T> {
    fn for_each<F: FnMut(&SisoFilter)>(&self, mut f: F) {
        for filter in self.iter() {
            f(filter.as_ref());
        }
    }

    fn for_each_mut<F: FnMut(&mut SisoFilter)>(&mut self, mut f: F) {
        for filter in self.iter_mut() {
            f(filter.as_mut());
        }
    }
}

macro_rules! impl_tuple_siso_filters {
    ( $(($idx:tt, $name:ident)),* ) => {
        impl<$($name),*> SisoFilters for ($($name,)*)
        where
            $($name: SisoFilter),* {
            fn for_each<F: FnMut(&SisoFilter)>(&self, mut f: F) {
                $(
                    f(&self.$idx);
                )*
            }

            fn for_each_mut<F: FnMut(&mut SisoFilter)>(&mut self, mut f: F) {
                $(
                    f(&mut self.$idx);
                )*
            }
        }
    }
}

impl_tuple_siso_filters! {
    (0, T1)
}
impl_tuple_siso_filters! {
    (0, T1), (1, T2)
}
impl_tuple_siso_filters! {
    (0, T1), (1, T2), (2, T3)
}
impl_tuple_siso_filters! {
    (0, T1), (1, T2), (2, T3), (3, T4)
}
impl_tuple_siso_filters! {
    (0, T1), (1, T2), (2, T3), (3, T4), (4, T5)
}
impl_tuple_siso_filters! {
    (0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6)
}
impl_tuple_siso_filters! {
    (0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6), (6, T7)
}
impl_tuple_siso_filters! {
    (0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6), (6, T7), (7, T8)
}

/// SISO filter that applies multiple `SisoFilter`s in a serial fashion.
///
/// # Examples
///
/// Thw following code shows how to create a `CascadedSisoFilter` from multiple
/// `SisoFilter`s.
///
///     # use ysr2_filters::siso::{IdentityFilter, SisoFilter, CascadedSisoFilter};
///     # use ysr2_filters::biquad::{BiquadCoefs, SimpleBiquadKernel};
///     # let coef = BiquadCoefs::identity();
///     CascadedSisoFilter::new(vec![
///         Box::new(IdentityFilter) as Box<SisoFilter>,
///         Box::new(SimpleBiquadKernel::new(&coef, 1)) as Box<SisoFilter>,
///     ]);
///
/// Or you could use a tuple instead for more efficiency:
///
///     # use ysr2_filters::siso::{IdentityFilter, SisoFilter, CascadedSisoFilter};
///     # use ysr2_filters::biquad::{BiquadCoefs, SimpleBiquadKernel};
///     # let coef = BiquadCoefs::identity();
///     CascadedSisoFilter::new((
///         IdentityFilter,
///         SimpleBiquadKernel::new(&coef, 1),
///     ));
///
pub struct CascadedSisoFilter<T>(T);

impl<T: SisoFilters> CascadedSisoFilter<T> {
    /// Construct a `CascadedSisoFilter`.
    ///
    /// The number of channels of every element of `filters` must match.
    pub fn new(filters: T) -> Self {
        let mut num_channels = None;
        filters.for_each(|filter| {
            let flt_num_channels = filter.num_channels();
            num_channels = num_channels.or(flt_num_channels);
            assert_eq!(num_channels, flt_num_channels.or(num_channels));
        });
        CascadedSisoFilter(filters)
    }
}

impl<T> CascadedSisoFilter<T> {
    /// Get a reference to the contained `SisoFilters`.
    pub fn get_ref(&self) -> &T {
        &self.0
    }

    /// Get a mutable reference to the contained `SisoFilters`.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }

    /// Consume this `CascadedSisoFilter` and return the contained `SisoFilters`.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: SisoFilters> SisoFilter for CascadedSisoFilter<T> {
    fn num_channels(&self) -> Option<usize> {
        let mut num_channels = None;
        self.0.for_each(|filter| if num_channels.is_none() {
            num_channels = filter.num_channels();
        });
        num_channels
    }
}

impl<T: SisoFilters> Filter for CascadedSisoFilter<T> {
    fn render(
        &mut self,
        to: &mut [&mut [f32]],
        range: Range<usize>,
        mut from: Option<(&[&[f32]], Range<usize>)>,
    ) {
        self.0.for_each_mut(|filter| {
            filter.render(to, range.clone(), from.take());
        });

        if let Some((input, ref in_range)) = from {
            assert_eq!(range.len(), in_range.len());
            assert_eq!(input.len(), to.len());
            for (to, from) in to.iter_mut().zip(input.iter()) {
                to[range.clone()].copy_from_slice(&from[in_range.clone()]);
            }
        }
    }

    fn is_active(&self) -> bool {
        let mut is_active = false;
        self.0.for_each(|filter| if filter.is_active() {
            is_active = true;
        });
        is_active
    }

    fn num_input_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn num_output_channels(&self) -> Option<usize> {
        self.num_channels()
    }

    fn skip(&mut self, num_samples: usize) {
        let num_channels = self.num_channels().unwrap();
        let mut buffer: Vec<Vec<f32>> = Vec::new();
        let mut found_active = false;

        self.0.for_each_mut(|filter| {
            if !found_active {
                // Try to use fast path as far as we can go
                if !filter.is_active() {
                    filter.skip(num_samples);
                    return;
                }
                found_active = false;
            }

            // If there are active filters in the middle, we need a temporary buffer
            if buffer.len() != num_channels {
                // Allocate a temporary buffer
                buffer = vec![vec![0.0; num_samples]; num_channels];
            }

            let mut buffer_refs: Vec<_> = buffer.iter_mut().map(Vec::as_mut_slice).collect();
            filter.render_inplace(buffer_refs.as_mut_slice(), 0..num_samples);
        });
    }

    fn reset(&mut self) {
        self.0.for_each_mut(|filter| { filter.reset(); });
    }
}
