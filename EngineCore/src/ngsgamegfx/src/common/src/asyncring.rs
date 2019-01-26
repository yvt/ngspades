//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a `Future`-compatible wrapper of `xalloc::Ring` that blocks if there isn't
//! a sufficient space.
use futures::{self, try_ready, Future, Poll};
use std::{collections::VecDeque, marker::Unpin, pin::Pin};
use xalloc::{int::BinaryUInteger, Ring, RingRegion};

/// A `Future`-compatible wrapper of `xalloc::Ring` that blocks if there isn't a
/// sufficient space.
#[derive(Debug)]
pub struct AsyncRing<F, T> {
    ring: Ring<T>,
    regions: VecDeque<Region<F, T>>,
}

#[derive(Debug)]
struct Region<F, T> {
    task: Option<F>,
    region: RingRegion<T>,
}

/// Represents an asynchronous operation inserting zero or more subregions into
/// `AsyncRing`.
///
/// The set of subregions inserted using a single `AllocBack` is
/// called a region (represented by `Region`) and associated with a single
/// `Future`.
#[derive(Debug)]
pub struct AllocBack<'a, F, T> {
    ring: &'a mut AsyncRing<F, T>,
    /// `true` if `ring.regions.back()` corresponds to the currently allocated
    /// region. (We don't create `Region` until there is at least one subregion.)
    has_region: bool,
}

/// Represents an asynchronous operation inserting one subregion into
/// `AsyncRing`.
///
/// The output is `Option<T>` and represents the offset of the allocated
/// region. `None` indicates that the allocation was unsuccessful.
#[derive(Debug)]
pub struct AllocBackSub<'a, 'b, F, T> {
    parent: &'a mut AllocBack<'b, F, T>,
    size: T,
    align: T,
    state: bool,
}

impl<F, T> AsyncRing<F, T>
where
    T: BinaryUInteger,
{
    pub fn new(size: T) -> Self {
        Self {
            ring: Ring::new(size),
            regions: VecDeque::new(),
        }
    }
}

impl<F, T> AsyncRing<F, T> {
    pub fn alloc_back_multi(&mut self) -> AllocBack<'_, F, T> {
        AllocBack {
            ring: self,
            has_region: false,
        }
    }
}

impl<'a, F, T> AllocBack<'a, F, T> {
    pub fn alloc_back_aligned(&mut self, size: T, align: T) -> AllocBackSub<'_, 'a, F, T> {
        AllocBackSub {
            parent: self,
            size,
            align,
            state: true,
        }
    }

    /// Assign a `Future` to the subregions allocated via this `AllocBack`.
    /// The future completes when the subregions can be deallocated.
    pub fn finish(self, future: F) {
        if self.has_region {
            self.ring.regions.back_mut().unwrap().task = Some(future);
        }
    }
}

impl<F, E, T> Future for AllocBackSub<'_, '_, F, T>
where
    F: Future<Output = Result<(), E>> + Unpin,
    T: BinaryUInteger + Unpin,
{
    type Output = Result<Option<T>, E>;

    fn poll(self: Pin<&mut Self>, lw: &futures::task::LocalWaker) -> Poll<Self::Output> {
        let this = self.get_mut(); // because `Self: Unpin`
        let async_ring: &mut AsyncRing<F, T> = &mut *this.parent.ring;

        loop {
            if this.state {
                // Allocate a region
                if let Some((region, offset)) = async_ring
                    .ring
                    .alloc_back_aligned(this.size.clone(), this.align.clone())
                {
                    if this.parent.has_region {
                        async_ring.regions.back_mut().unwrap().region = region;
                    } else {
                        async_ring.regions.push_back(Region { task: None, region });
                        this.parent.has_region = true;
                    }
                    return Poll::Ready(Ok(Some(offset)));
                }

                this.state = false;
            }

            // Allocation failed. Try deallocating the oldest region
            if let Some(region) = async_ring.regions.front_mut() {
                if let Some(ref mut task) = region.task {
                    try_ready!(Pin::new(task).poll(lw));
                } else {
                    return Poll::Ready(Ok(None));
                }
            } else {
                return Poll::Ready(Ok(None));
            }

            let region = async_ring.regions.pop_front().unwrap();
            async_ring.ring.dealloc_front_until(region.region.clone());
            async_ring.ring.dealloc_front(region.region);

            this.state = true;
        }
    }
}
