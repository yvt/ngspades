//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Memory allocators for argument pool.
use xalloc::{SysTlsf, SysTlsfRegion};

use super::ArgSize;

pub type Allocation = Option<SysTlsfRegion>;

pub trait Allocator: crate::Debug + Send + Sync + Sized {
    fn new(size: ArgSize) -> Self;
    fn allocate(&mut self, size: ArgSize, align: ArgSize) -> Option<(ArgSize, Allocation)>;
    fn deallocate(&mut self, p: Allocation);
    fn reset(&mut self);
}

#[derive(Debug)]
pub struct StackAllocator {
    allocated: ArgSize,
    size: ArgSize,
}

impl Allocator for StackAllocator {
    fn new(size: ArgSize) -> Self {
        Self { allocated: 0, size }
    }

    fn allocate(&mut self, size: ArgSize, align: ArgSize) -> Option<(ArgSize, Allocation)> {
        let mut new_allocated = self.allocated;

        // Insert a pad to meet the alignment requirement
        new_allocated = (new_allocated + align - 1) & !(align - 1);
        if new_allocated < self.allocated {
            return None;
        }

        // Push it into the stack and see if it overflows
        let start = new_allocated;
        new_allocated = if let Some(x) = new_allocated.checked_add(size) {
            x
        } else {
            return None;
        };

        if new_allocated > self.size {
            return None;
        }

        self.allocated = new_allocated;
        Some((start, None))
    }

    fn deallocate(&mut self, _: Allocation) {
        // `StackAllocator` does not support deallocation.
    }

    fn reset(&mut self) {
        self.allocated = 0;
    }
}

#[derive(Debug)]
pub struct TlsfAllocator {
    tlsf: SysTlsf<ArgSize>,
    size: ArgSize,
}

impl Allocator for TlsfAllocator {
    fn new(size: ArgSize) -> Self {
        Self {
            tlsf: SysTlsf::new(size),
            size,
        }
    }

    fn allocate(&mut self, size: ArgSize, align: ArgSize) -> Option<(ArgSize, Allocation)> {
        self.tlsf
            .alloc_aligned(size, align)
            .map(|(alloc, offset)| (offset, Some(alloc)))
    }

    fn deallocate(&mut self, p: Allocation) {
        // The application has to follow the valid usage for this to be safe
        unsafe {
            self.tlsf.dealloc_unchecked(p.unwrap());
        }
    }

    fn reset(&mut self) {
        // I'd imagine this is extremely inefficient...
        self.tlsf = SysTlsf::new(self.size);
    }
}
