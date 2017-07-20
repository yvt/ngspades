//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a dynamic external memory suballocator.
use int::{BinaryInteger, BinaryUInteger};
use num_traits::{Zero, One};
use pool::{PoolFreePtr, Pool};

type TlsfL2Bitmap = u16;
const LOG2_L2_SIZE: u32 = 4; // must be <= log2(sizeof(TlsfL2Bitmap)*8)
const L2_SIZE: u32 = 1 << LOG2_L2_SIZE;

/// A dynamic external memory suballocator implemented with the TLSF
/// (Two-Level Segregated Fit) algorithm.
///
/// This TLSF implements a Good-Fit strategy. In order to achieve the O(1)
/// execution time, only the first element of each free space list is examined.
/// As a result, allocations are not guaranteed to succeed even if there
/// is an enough free space if all of the following conditions are met:
///
///  - There is a free space that is only slightly smaller than the requested
///    size.
///  - There is no free space that is larger by some certain amount than
///    the requested size.
///
/// Performance
/// -----------
///
/// Don't even think about using this as an alternative system memory allocator.
#[derive(Debug)]
pub struct TlsfSuballoc<T: BinaryUInteger> {
    size: T,
    l1: TlsfL1<T>,
    blocks: Pool<TlsfBlock<T>>,
}

/// A handle type to a region allocated in a `TlsfSuballoc`.
///
/// `TlsfSuballocRegion` returned by a `TlsfSuballoc` only can be used with the
/// same `TlsfSuballoc`. It will be invalidated when the region was freed by
/// calling `TlsfSuballoc::deallocate`.
/// Otherwise, `TlsfSuballoc` might behave in an unpredictable way.
/// (It is still not `unsafe` by itself, though)
///
/// `Eq` only makes sense between valid objects from the same `TlsfSuballoc`.
///
/// Does not implement `Copy` because the only possible operation on
/// a `TlsfSuballocRegion` is deallocation, which essentially invalidates the
/// given region handle. `Clone`-ing is allowed but is unlikely to make a sense.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TlsfSuballocRegion(PoolFreePtr);

#[derive(Debug)]
struct TlsfBlock<T: BinaryUInteger> {
    /// Points the previous (in terms of the external memory address) block.
    prev: Option<PoolFreePtr>,

    /// Points the next (in terms of the external memory address) block.
    next: Option<PoolFreePtr>,

    /// The external memory address.
    address: T,

    /// The size of the block in the external memory space.
    size: T,
    state: TlsfBlockState,
}

#[derive(Debug, PartialEq, Eq)]
enum TlsfBlockState {
    Free {
        /// The previous free block in the same free space list.
        prev_free: Option<PoolFreePtr>,

        /// The next free block in the same free space list.
        next_free: Option<PoolFreePtr>,
    },
    Used,
}

/// First level table.
#[derive(Debug)]
struct TlsfL1<T: BinaryUInteger> {
    /// Array of second level tables.
    ///
    /// - `l1[0]` contains segregated lists for free spaces smaller
    ///   than `L2_SIZE`.
    ///   `l1[0].l2[L] contains the segregated list for free spaces whose sizes
    ///   are equal to `L`.
    /// - `l1[K]` contains segregated lists for free spaces whose sizes are
    ///   in the range `L2_SIZE << (K - 1) .. L2_Size << K`.
    ///   `l1[K].l2[L] contains the segregated list for free spaces whose sizes
    ///   are in the range
    ///   `(L2_SIZE << (K - 1)) + (1 << (K - 1)) * L .. (L2_Size << (K - 1)) + (1 << (K - 1)) * (L + 1)`
    ///
    l1: Vec<TlsfL2>,

    /// Each bit indices whether the corresponding element of
    /// `l1` has at least one free space or not.
    ///
    /// The following invariant holds:
    ///
    ///  - `(bitmap.extract_u32(i..(i+1)) != 0) == (i1[i].bitmap != 0)`
    //
    /// The number of L2 tables is proportional to the number of digits of the pool
    /// size, so using `T` here would be a good choice.
    bitmap: T,

    /// Points the free block that fills entire the available space
    /// (used only if the pool size is a power of two and no
    /// segregated list entry is available for it)
    entire: Option<PoolFreePtr>,
}

/// Second level table.
#[derive(Debug, Clone)]
struct TlsfL2 {
    /// Each bit indicates whether the corresponding element of
    /// `l2` is valid or not.
    bitmap: TlsfL2Bitmap,

    /// Each element represents the first block in a free space list.
    ///
    /// Points blocks stored in `TlsfSuballoc::blocks`. The validity of each
    /// element is indicated by the corresponding bit of `bitmap`.
    l2: [PoolFreePtr; L2_SIZE as usize],
}

impl<T: BinaryUInteger> TlsfSuballoc<T> {
    pub fn new(size: T) -> Self {
        Self::with_capacity(size, 1)
    }

    /// Construct a `TlsfSuballoc` with a pool capacity.
    pub fn with_capacity(size: T, num_blocks: usize) -> Self {
        let mut sa = TlsfSuballoc {
            l1: TlsfL1::new(&size),
            size: size,
            blocks: Pool::with_capacity(num_blocks),
        };

        // Create the initial free block
        let block = TlsfBlock {
            prev: None,
            next: None,
            address: Zero::zero(),
            size: sa.size.clone(),
            state: TlsfBlockState::Used, // don't care
        };
        let block_ptr = sa.blocks.allocate(block);
        sa.l1.link(&mut sa.blocks, block_ptr);

        sa
    }

    /// Allocate a region of the size `size` with a given alignment requirement.
    ///
    /// Returns a handle of the allocated region and its offset if the
    /// allocation succeeds. Returns `None` otherwise.
    ///
    /// `align` must be a power of two.
    ///
    /// `size` must not be zero.
    pub fn allocate_aligned(&mut self, size: T, align: T) -> Option<(TlsfSuballocRegion, T)> {
        assert!(align.is_power_of_two());
        self.allocate_aligned_log2(size, align.trailing_zeros())
    }

    /// Allocate a region of the size `size`.
    ///
    /// Returns a handle of the allocated region and its offset if the
    /// allocation succeeds. Returns `None` otherwise.
    ///
    /// `size` must not be zero.
    pub fn allocate(&mut self, size: T) -> Option<(TlsfSuballocRegion, T)> {
        self.allocate_aligned_log2(size, 0)
    }

    fn allocate_aligned_log2(
        &mut self,
        size: T,
        align_bits: u32,
    ) -> Option<(TlsfSuballocRegion, T)> {
        if size > self.size {
            return None;
        }
        assert_ne!(size, Zero::zero());

        let suitable = self.l1.search_suitable(&mut self.blocks, &size, align_bits);
        suitable.map(|(position, free_block_ptr, pad)| {
            let (mut prev, mut next, free_block_address, free_block_size) = {
                let ref block: TlsfBlock<T> = self.blocks[free_block_ptr];
                (
                    block.prev,
                    block.next,
                    block.address.clone(),
                    block.size.clone(),
                )
            };
            let data_end = pad.clone() + size.clone();

            // For exception safety...
            let mut reserve = 0;
            if pad != Zero::zero() {
                reserve += 1;
            }
            if data_end != free_block_size {
                reserve += 1;
            }
            self.blocks.reserve(reserve);

            self.l1.unlink_head(
                &mut self.blocks,
                free_block_ptr,
                position,
            );
            self.blocks.deallocate(free_block_ptr);

            if pad != Zero::zero() {
                let block = TlsfBlock {
                    prev,
                    next: None, // linked later
                    address: free_block_address.clone(),
                    size: pad.clone(),
                    state: TlsfBlockState::Used, // don't care
                };
                let block_ptr = self.blocks.allocate(block);
                self.l1.link(&mut self.blocks, block_ptr);
                if let Some(old_prev) = prev {
                    self.blocks[old_prev].next = Some(block_ptr);
                }
                prev = Some(block_ptr);
            }

            if data_end != free_block_size {
                let block = TlsfBlock {
                    prev: None, // linked later
                    next,
                    address: free_block_address.clone() + data_end.clone(),
                    size: free_block_size.clone() - data_end.clone(),
                    state: TlsfBlockState::Used, // don't care
                };
                let block_ptr = self.blocks.allocate(block);
                self.l1.link(&mut self.blocks, block_ptr);
                if let Some(old_next) = next {
                    self.blocks[old_next].prev = Some(block_ptr);
                }
                next = Some(block_ptr);
            }

            let main_ptr = {
                let block = TlsfBlock {
                    prev,
                    next,
                    address: free_block_address.clone() + pad.clone(),
                    size,
                    state: TlsfBlockState::Used, // care!
                };
                let block_ptr = self.blocks.allocate(block);
                block_ptr
            };

            // Connect neighboring blocks to this
            let address = self.blocks[main_ptr].address.clone();

            if let Some(ptr) = prev {
                self.blocks[ptr].next = Some(main_ptr);
            }
            if let Some(ptr) = next {
                self.blocks[ptr].prev = Some(main_ptr);
            }


            (TlsfSuballocRegion(main_ptr), address)
        })
    }

    /// Deallocate the specified region.
    pub fn deallocate(&mut self, r: TlsfSuballocRegion) {
        let block_ptr = r.0;

        let (prev_ptr, next_ptr) = {
            let ref block: TlsfBlock<T> = self.blocks[block_ptr];
            assert_eq!(block.state, TlsfBlockState::Used);
            (block.prev, block.next)
        };


        // Try to merge neighboring free blocks
        let prev_info = if let Some(ptr) = prev_ptr {
            let ref block: TlsfBlock<T> = self.blocks[ptr];
            if let TlsfBlockState::Free { .. } = block.state {
                Some((block.prev, block.size.clone()))
            } else {
                None
            }
        } else {
            None
        };
        let next_info = if let Some(ptr) = next_ptr {
            let ref block: TlsfBlock<T> = self.blocks[ptr];
            if let TlsfBlockState::Free { .. } = block.state {
                Some((block.next, block.size.clone()))
            } else {
                None
            }
        } else {
            None
        };
        {
            let ref mut block: TlsfBlock<T> = self.blocks[block_ptr];
            if let Some((ref new_prev_ptr, ref prev_size)) = prev_info {
                block.prev = *new_prev_ptr;
                block.size += prev_size.clone();
                block.address -= prev_size.clone();
            }
            if let Some((ref new_next_ptr, ref next_size)) = next_info {
                block.next = *new_next_ptr;
                block.size += next_size.clone();
            }
        }

        if prev_info.is_some() {
            self.l1.unlink(&mut self.blocks, prev_ptr.unwrap());
            self.blocks.deallocate(prev_ptr.unwrap());
        }
        if next_info.is_some() {
            self.l1.unlink(&mut self.blocks, next_ptr.unwrap());
            self.blocks.deallocate(next_ptr.unwrap());
        }

        if let Some((Some(new_prev_ptr), _)) = prev_info {
            let ref mut block: TlsfBlock<T> = self.blocks[new_prev_ptr];
            block.next = Some(block_ptr);
        }
        if let Some((Some(new_next_ptr), _)) = next_info {
            let ref mut block: TlsfBlock<T> = self.blocks[new_next_ptr];
            block.prev = Some(block_ptr);
        }

        self.l1.link(&mut self.blocks, block_ptr);
    }

    #[doc(hidden)]
    pub fn test_integrity(&mut self, root_ptr: &TlsfSuballocRegion) {
        // Find the physically first block
        let mut first_ptr = root_ptr.0;
        while self.blocks[first_ptr].prev.is_some() {
            first_ptr = self.blocks[first_ptr].prev.unwrap();
        }

        let dump = || {
            use std::fmt::Write;
            let mut s = String::new();
            let mut cur_ptr = first_ptr;
            loop {
                let ref cur = self.blocks[cur_ptr];
                let next_ptr = cur.next;
                write!(
                    &mut s,
                    "{:?} - [{:?}, {:?}] - {:?}\n",
                    cur.prev,
                    cur_ptr,
                    cur.state,
                    cur.next
                ).unwrap();
                if let Some(next_ptr) = next_ptr {
                    cur_ptr = next_ptr;
                } else {
                    break;
                }
            }
            s
        };

        // scan every block and check the physical connections
        let mut cur_ptr = first_ptr;
        let mut addr = Zero::zero();
        loop {
            let ref cur = self.blocks[cur_ptr];
            assert_eq!(
                cur.address,
                addr,
                "[{:?}].prev ({:?}) should be {:?}. Dump: \n{}",
                cur_ptr,
                &cur.address,
                &addr,
                dump()
            );
            addr += cur.size.clone();

            let next_ptr = cur.next;
            if let Some(next_ptr) = next_ptr {
                let ref next = self.blocks[next_ptr];
                assert_eq!(
                    next.prev,
                    Some(cur_ptr),
                    "[{:?}].prev ({:?}) should be {:?}. Dump: \n{}",
                    next_ptr,
                    next.prev,
                    cur_ptr,
                    dump()
                );
                assert!(
                    next.state == TlsfBlockState::Used || cur.state == TlsfBlockState::Used,
                    "[{:?}].state and [{:?}].state must not be Free at the same time. Dump: \n{}",
                    next_ptr,
                    cur_ptr,
                    dump()
                );
                cur_ptr = next_ptr;
            } else {
                break;
            }
        }
        assert_eq!(
            self.size,
            addr,
            "self.size ({:?}) should be {:?}. Dump: \n{}",
            &self.size,
            &addr,
            dump()
        );
    }
}

impl<T: BinaryUInteger> TlsfBlock<T> {
    /// Return whether the requested region can fit in this space (assuming it
    /// is free).
    ///
    /// The returned value is the size of padding required to meet the
    /// alignment requirement. `None` if it cannot fit.
    fn can_fit(&self, size: &T, align_bits: u32) -> Option<T> {
        if align_bits == 0 {
            if size <= &self.size {
                Some(Zero::zero())
            } else {
                None
            }
        } else {
            let start = self.address.clone().checked_ceil_fix(align_bits);
            let end_block = self.address.clone() + self.size.clone();
            if let Some(start) = start {
                if start < end_block && size <= &(end_block.clone() - start.clone()) {
                    Some(start - self.address.clone())
                } else {
                    None
                }
            } else {
                start
            }
        }
    }
}

impl<T: BinaryUInteger> TlsfL1<T> {
    /// Constructs `TlsfL1`.
    fn new(size: &T) -> Self {
        assert!(size > &Zero::zero());

        let size_m1 = size.clone() - One::one();
        let num_l2s = T::max_digits().saturating_sub(LOG2_L2_SIZE + size_m1.leading_zeros()) + 1;

        Self {
            l1: vec![
                TlsfL2 {
                    bitmap: Zero::zero(),
                    l2: [PoolFreePtr::uninitialized(); L2_SIZE as usize],
                };
                num_l2s as usize
            ],
            bitmap: Zero::zero(),
            entire: None,
        }
    }

    /// Compute the first and second level table index for a given size of free
    /// space.
    fn map_size(&self, size: &T) -> (u32, u32) {
        let l1_index = T::max_digits().saturating_sub(LOG2_L2_SIZE + size.leading_zeros());
        let min_bit_index = l1_index.saturating_sub(1);
        let l2_index = size.extract_u32(min_bit_index..min_bit_index + LOG2_L2_SIZE);
        (l1_index, l2_index)
    }

    /// Search a free block at least as large as `size` with the alignment
    /// requirement `1 << align_bits`.
    ///
    /// The result can be one of the following:
    ///
    ///  - `None`: No suitable block was found.
    ///  - `Some((position, block_ptr, pad)):  A suitable block was found. `position` is either of:
    ///      - `Some((l1, l2))`: `block_ptr` is the head of the free space list at the position `(l1, l2)`.
    ///      - `None`: `block_ptr` is `self.entire`.
    ///
    /// `size` must be less than or equal to the size of the heap.
    fn search_suitable(
        &self,
        blocks: &mut Pool<TlsfBlock<T>>,
        size: &T,
        align_bits: u32,
    ) -> Option<(Option<(u32, u32)>, PoolFreePtr, T)> {
        if let Some(entire) = self.entire {
            return Some((None, entire, Zero::zero()));
        }

        let (l1_first, l2_first) = self.map_size(size);
        if self.bitmap.get_bit(l1_first) {
            let ref l2t: TlsfL2 = self.l1[l1_first as usize];
            if l2t.bitmap.get_bit(l2_first) {
                // Found a free block in the same bucket.
                let block_ptr = l2t.l2[l2_first as usize];
                let ref block = blocks[block_ptr];
                if let Some(pad) = block.can_fit(size, align_bits) {
                    return Some((Some((l1_first, l2_first)), block_ptr, pad));
                }
            }

            // Search the same second level table.
            let l2 = l2t.bitmap.bit_scan_forward(l2_first + 1);
            if l2 != TlsfL2Bitmap::max_digits() {
                // Found one
                let block_ptr = l2t.l2[l2 as usize];
                let can_fit = if align_bits == 0 {
                    Some(Zero::zero())
                } else {
                    blocks[block_ptr].can_fit(size, align_bits)
                };
                if let Some(pad) = can_fit {
                    if align_bits == 0 {
                        debug_assert!(blocks[block_ptr].can_fit(size, align_bits).is_some());
                    }
                    return Some((Some((l1_first, l2)), block_ptr, pad));
                }
            }
        }

        let mut l1_first = self.bitmap.bit_scan_forward(l1_first + 1);
        let mut l2_first = if l1_first == T::max_digits() {
            return None;
        } else {
            let ref l2t: TlsfL2 = self.l1[l1_first as usize];
            let l2 = l2t.bitmap.bit_scan_forward(0);
            debug_assert_ne!(l2, TlsfL2Bitmap::max_digits());
            let block_ptr = l2t.l2[l2 as usize];
            let can_fit = if align_bits == 0 {
                Some(Zero::zero())
            } else {
                blocks[block_ptr].can_fit(size, align_bits)
            };
            if let Some(pad) = can_fit {
                if align_bits == 0 {
                    debug_assert!(blocks[block_ptr].can_fit(size, align_bits).is_some());
                }
                return Some((Some((l1_first, l2)), block_ptr, pad));
            }
            l2
        };

        // For aligned allocations, there are cases where no free space that can
        // satisfy the alignment requirement even if the size requirement is met.
        // We need to check more free lists.
        //
        // The code below should be unreachable for allocations without an
        // alignment requirement.
        debug_assert_ne!(align_bits, 0);

        // FIXME: add explanation
        let worst_size = size.ref_saturating_add(T::ones(0..align_bits));
        let (l1_worst, l2_worst) = self.map_size(&worst_size);
        while (l1_first, l2_first) < (l1_worst, l2_worst) {
            // Determine the next search start position
            l2_first += 1;
            if l2_first >= TlsfL2Bitmap::max_digits() {
                l1_first = self.bitmap.bit_scan_forward(l1_first + 1);
                if l1_first == T::max_digits() {
                    return None;
                }
                l2_first = 0;
            }

            let ref l2t: TlsfL2 = self.l1[l1_first as usize];
            let l2 = l2t.bitmap.bit_scan_forward(l2_first);
            if l2 == TlsfL2Bitmap::max_digits() {
                l2_first = l2;
                continue;
            }
            let block_ptr = l2t.l2[l2 as usize];
            if let Some(pad) = blocks[block_ptr].can_fit(size, align_bits) {
                return Some((Some((l1_first, l2)), block_ptr, pad));
            } else {
                l2_first = l2;
            }
        }

        None
    }

    /// Remove the given block from the free space list.
    fn unlink(&mut self, blocks: &mut Pool<TlsfBlock<T>>, block_ptr: PoolFreePtr) {
        let (l1, l2) = self.map_size(&blocks[block_ptr].size);
        if l1 == self.l1.len() as u32 {
            debug_assert_eq!(Some(block_ptr), self.entire);
            self.entire = None;
        } else {
            {
                debug_assert!(self.bitmap.get_bit(l1));
                debug_assert!(
                    self.l1[l1 as usize].bitmap.get_bit(l2),
                    "L2 bitmap 0b{:b} has not bit {} set.",
                    &self.l1[l1 as usize].bitmap,
                    l2
                );
                if self.l1[l1 as usize].l2[l2 as usize] == block_ptr {
                    return self.unlink_head(blocks, block_ptr, Some((l1, l2)));
                }
            }

            // Retrieve the neighboring blocks (in the free space list)
            let (prev_ptr, o_next_ptr) = {
                let ref block = blocks[block_ptr];
                if let TlsfBlockState::Free {
                    prev_free: Some(prev_free),
                    next_free,
                } = block.state
                {
                    (prev_free, next_free)
                } else {
                    unreachable!()
                }
            };

            // Unlink the current block
            if let Some(next_ptr) = o_next_ptr {
                let ref mut next_block = blocks[next_ptr];
                if let TlsfBlockState::Free { ref mut prev_free, .. } = next_block.state {
                    debug_assert_eq!(*prev_free, Some(block_ptr));
                    *prev_free = Some(prev_ptr);
                } else {
                    unreachable!()
                }
            }

            {
                let ref mut prev_block = blocks[prev_ptr];
                if let TlsfBlockState::Free { ref mut next_free, .. } = prev_block.state {
                    debug_assert_eq!(*next_free, Some(block_ptr));
                    *next_free = o_next_ptr;
                } else {
                    unreachable!()
                }
            }
        }
    }

    /// Remove the given block from the free space list.
    ///
    /// `block_ptr` must be the head of the free space list specified by `position`.
    /// `block_ptr` returned by `search_suitable` always satisfies this condition,
    /// supposing no intervening modification was done.
    fn unlink_head(
        &mut self,
        blocks: &mut Pool<TlsfBlock<T>>,
        block_ptr: PoolFreePtr,
        position: Option<(u32, u32)>,
    ) {
        if let Some((l1, l2)) = position {
            let ref mut l2t: TlsfL2 = self.l1[l1 as usize];

            debug_assert!(self.bitmap.get_bit(l1));
            debug_assert!(
                l2t.bitmap.get_bit(l2),
                "L2 bitmap 0b{:b} has not bit {} set.",
                &l2t.bitmap,
                l2
            );
            debug_assert_eq!(block_ptr, l2t.l2[l2 as usize]);

            let next_block_ptr = {
                let ref block = blocks[block_ptr];
                if let TlsfBlockState::Free { next_free, .. } = block.state {
                    next_free
                } else {
                    unreachable!()
                }
            };

            if let Some(next_block_ptr) = next_block_ptr {
                let ref mut next_block = blocks[next_block_ptr];
                if let TlsfBlockState::Free { ref mut prev_free, .. } = next_block.state {
                    debug_assert_eq!(*prev_free, Some(block_ptr));
                    *prev_free = None;
                } else {
                    unreachable!()
                }

                l2t.l2[l2 as usize] = next_block_ptr;
            } else {
                l2t.bitmap.clear_bit(l2);
                if l2t.bitmap == Zero::zero() {
                    self.bitmap.clear_bit(l1);
                }

                // don't care about the value of `l2t.l2[l2 as usize]`
            }

        } else {
            debug_assert_eq!(Some(block_ptr), self.entire);
            self.entire = None;
        }
    }

    /// Insert the given block to a free space list.
    ///
    /// `block_ptr` must point a valid `TlsfBlock` in `blocks`.
    /// The given block's `TlsfBlock::state` will be overwritten with a new
    /// `TlsfBlockState::Free` value.
    fn link(&mut self, blocks: &mut Pool<TlsfBlock<T>>, block_ptr: PoolFreePtr) {
        let (l1, l2) = self.map_size(&blocks[block_ptr].size);
        if l1 == self.l1.len() as u32 {
            self.entire = Some(block_ptr);
        } else {
            let ref mut l2t: TlsfL2 = self.l1[l1 as usize];

            // Update bitmaps
            let head_valid = l2t.bitmap.get_bit(l2);
            l2t.bitmap.set_bit(l2);
            self.bitmap.set_bit(l1);

            // Link the given block to the list
            let mut head = &mut l2t.l2[l2 as usize];

            {
                let ref mut block = blocks[block_ptr];
                block.state = TlsfBlockState::Free {
                    prev_free: None,
                    next_free: if head_valid { Some(*head) } else { None },
                };
            }
            if head_valid {
                let ref mut next_block = blocks[*head];
                if let TlsfBlockState::Free { ref mut prev_free, .. } = next_block.state {
                    debug_assert!(prev_free.is_none());
                    *prev_free = Some(block_ptr);
                } else {
                    unreachable!()
                }
            }

            *head = block_ptr;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    struct Xorshift32(u32);

    impl Xorshift32 {
        fn next(&mut self) -> u32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 17;
            self.0 ^= self.0 << 5;
            !self.0
        }
    }

    #[test]
    fn num_l2s() {
        for i in 1..L2_SIZE {
            let l1 = TlsfL1::new(&(i as u32));
            assert_eq!(l1.l1.len(), 1);
        }
        for k in 0..4 {
            let i = L2_SIZE << k;
            let l1 = TlsfL1::new(&i);
            assert_eq!(l1.l1.len(), k + 1);
        }
    }

    #[bench]
    fn allocation_random_suballoc(b: &mut Bencher) {
        let mut v = vec![None; 512];
        let mut sa = TlsfSuballoc::with_capacity(512u32, 512);
        b.iter(|| {
            let mut r = Xorshift32(0x11451419);
            for _ in 0..65536 {
                let i = ((r.next() >> 8) & 511) as usize;
                if v[i].is_some() {
                    sa.deallocate(v[i].take().unwrap());
                } else {
                    v[i] = Some(sa.allocate(1u32).unwrap().0);
                }
            }
            for x in v.iter_mut() {
                if let Some(x) = x.take() {
                    sa.deallocate(x);
                }
            }
        });
    }
}
