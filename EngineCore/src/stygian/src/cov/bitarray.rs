//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Bit array.
use std::ops::Range;

use super::{CovBuffer, CovPainter};

/// A coverage buffer implemented using a bit array.
#[derive(Debug, Default, Clone)]
pub struct BitArray {
    buffer: Vec<usize>,
    len: u32,
}

/// The number of bits in `usize` (an element of `BitArray::buffer`).
const BITS: u32 = std::mem::size_of::<usize>() as u32 * 8;

unsafe impl CovBuffer for BitArray {
    fn reserve(&mut self, size: u32) {
        let count = (size.checked_add(BITS).unwrap() / BITS) as usize;
        if count > self.buffer.len() {
            self.buffer.reserve(count - self.buffer.len());
        }
    }

    fn resize(&mut self, size: u32) {
        // Allocate extra one element
        let count = (size.checked_add(BITS).unwrap() / BITS) as usize;
        self.buffer.clear();
        self.buffer.resize(count, !0);
        self.len = size;
    }

    fn len(&self) -> u32 {
        self.len
    }

    #[inline]
    unsafe fn paint<T: CovPainter>(&mut self, range: Range<u32>, mut painter: T) {
        let buffer = &mut self.buffer[..];
        let (mut start, end) = (range.start, range.end);
        let mut next_boundary = (start / BITS + 1) * BITS;

        debug_assert!(start < end, "{} < {}", start, end);
        debug_assert!(end <= self.len, "{} <= {}", end, self.len);

        // Make a sentry
        let oldval = {
            // It's okay not to check the bounds because `resize` allocates
            // one extra element
            let oldval = *buffer.get_unchecked((end / BITS) as usize) & !ones(end);
            *buffer.get_unchecked_mut((end / BITS) as usize) &= ones(end);
            oldval
        };

        debug_assert!(((start / BITS) as usize) < buffer.len());
        let mut bits = *buffer.get_unchecked((start / BITS) as usize);

        bits = bits.wrapping_shr(start);

        loop {
            // Skip the painted elements
            let count = bits.trailing_zeros();
            let next_start = start + count;

            if next_start >= next_boundary {
                if next_boundary >= end {
                    break;
                }
                painter.skip(next_boundary - start);
                start = next_boundary;

                // Read the next bitmap
                debug_assert!(((start / BITS) as usize) < buffer.len());
                bits = *buffer.get_unchecked((start / BITS) as usize);
                next_boundary += BITS;
                continue;
            }

            painter.skip(count);
            start = next_start;
            bits >>= count;

            // Paint the unpainted elements
            let mut count = (!bits).trailing_zeros();
            debug_assert!(count > 0);

            // Consume the painted bits. Note that `count` can be equal to `BITS`
            bits >>= 1;
            bits >>= count - 1;

            // Mark as painted. Make sure to RMW on `buffer` so that a processor
            // can do this asynchronously
            *buffer.get_unchecked_mut((start / BITS) as usize) &= !ones2(count).wrapping_shl(start);

            // Advance the state
            loop {
                // Prevent loop unrolling.
                asm!("");

                debug_assert!(start < end);
                painter.paint(start);
                start += 1;
                count -= 1;
                if count == 0 {
                    break;
                }
            }
        }

        // Restore the sentry
        *buffer.get_unchecked_mut((end / BITS) as usize) |= oldval;
    }
}

#[inline]
fn one(x: u32) -> usize {
    1usize.wrapping_shl(x)
}
// [0, x % BITS]
//
// |  x | out                                 |
// | -- | ----------------------------------- |
// |  0 | 00000000 00000000 00000000 00000000 |
// |  8 | 00000000 00000000 00000000 11111111 |
// | 31 | 11111111 11111111 11111111 11111111 |
// | 32 | 00000000 00000000 00000000 00000000 |
#[inline]
fn ones(x: u32) -> usize {
    one(x) - 1
}
// [0, (x - 1) % BITS + 1]
//
// |  x | out                                 |
// | -- | ----------------------------------- |
// |  1 | 00000000 00000000 00000000 00000001 |
// | 31 | 01111111 11111111 11111111 11111111 |
// | 32 | 11111111 11111111 11111111 11111111 |
#[inline]
fn ones2(x: u32) -> usize {
    2usize.wrapping_shl(x.wrapping_sub(1)).wrapping_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_bit_seq(x: std::ops::Range<u32>) -> usize {
        x.map(|i| 1usize << i).fold(0, |x, y| x | y)
    }

    #[test]
    fn one_sanity() {
        assert_eq!(one(0), 1usize);
        assert_eq!(one(BITS - 1), 1usize << (BITS - 1));
    }

    #[test]
    fn ones_sanity() {
        for k in 0..BITS * 2 {
            dbg!(k);
            assert_eq!(ones(k), naive_bit_seq(0..k % BITS));
        }
    }

    #[test]
    fn ones2_sanity() {
        for k in 0..BITS * 2 {
            dbg!(k);
            assert_eq!(ones2(k), naive_bit_seq(0..(k + BITS - 1) % BITS + 1));
        }
    }
}
