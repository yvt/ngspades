//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ngsgfx_common::suballoc::{TlsfSuballoc, TlsfSuballocRegion};

#[derive(Debug)]
pub struct Suballocator(TlsfSuballoc<u64>);

#[derive(Debug)]
pub struct SuballocatorRegion(Option<TlsfSuballocRegion>, u64, u64);

impl PartialEq for SuballocatorRegion {
    fn eq(&self, other: &Self) -> bool {
        self.0.is_some() && self.0 == other.0
    }
}

impl Suballocator {
    pub fn new(size: u64) -> Self {
        Suballocator(TlsfSuballoc::new(size))
    }
    pub fn allocate(&mut self, size: u64, align: u64) -> Option<SuballocatorRegion> {
        self.0.allocate_aligned(size, align).map(
            |(handle, offset)| {
                SuballocatorRegion(Some(handle), offset, size)
            },
        )
    }

    /// Deallocate a region. `region` must have been allocated from the
    /// same `Suballocator`.
    pub fn deallocate(&mut self, mut region: SuballocatorRegion) {
        self.make_aliasable(&mut region);
    }

    pub fn make_aliasable(&mut self, region: &mut SuballocatorRegion) {
        if let Some(r) = region.0.take() {
            self.0.deallocate(r);
        }
    }
}

impl SuballocatorRegion {
    pub fn offset(&self) -> u64 {
        self.1
    }
    pub fn size(&self) -> u64 {
        self.2
    }
}
