//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Skip buffer.
use std::{ops::Range, mem::replace};

use super::{CovBuffer, CovPainter};

/// A skip buffer.
#[derive(Debug, Default, Clone)]
pub struct SkipBuffer {
    buffer: Vec<u32>,
}

/// In a skip buffer, this flag indicates there are no more vacant elements
/// afterward.
///
/// Why use a flag instead of a specific value? Some of x86's status registers
/// update automatically based on MSB. See <https://godbolt.org/z/tqCzQC>
/// (As it turned out be, this approach didn't work out in most cases..)
///
/// Why `u32`? x86 doesn't allow 16-bit registers for indexed addressing!
const EOB_BIT: u32 = 1 << 31;

unsafe impl CovBuffer for SkipBuffer {
    fn reserve(&mut self, size: u32) {
        let buffer = &mut self.buffer;

        // Skip buffer would overflow if `size` is too large
        assert!(size <= 0x40000000, "beam depth buffer is too large");

        let size = size as usize + 1;
        if size > buffer.len() {
            buffer.reserve(size - buffer.len());
        }
    }

    fn resize(&mut self, size: u32) {
        let buffer = &mut self.buffer;

        // Skip buffer would overflow if `size` is too large
        assert!(size <= 0x40000000, "beam depth buffer is too large");

        buffer.clear();
        buffer.resize(size as usize + 1, 0);
        buffer[size as usize] = EOB_BIT;
    }

    fn len(&self) -> u32 {
        self.buffer.len().checked_sub(1).unwrap() as u32
    }

    #[inline]
    unsafe fn paint<T: CovPainter>(&mut self, range: Range<u32>, mut painter: T) {
        let buffer = &mut self.buffer[..];

        if range.start >= range.end {
            return;
        }

        let mut i = range.start;

        // Temporarily replace the end value with a sentry.
        // This approach saves one comparison after `i += 1`, slightly improving
        // the throughput of the fill loop (by roughly 15% on SKL).
        let end_skip = replace(&mut *buffer.get_unchecked_mut(range.end as usize), EOB_BIT);

        let mut skip = *buffer.get_unchecked(i as usize);

        loop {
            if skip != 0 {
                i += skip;
                if i >= range.end {
                    break;
                }
                painter.skip(skip);
                skip = *buffer.get_unchecked(i as usize);
                continue;
            }

            loop {
                *buffer.get_unchecked_mut(i as usize) = end_skip + (range.end - i);
                painter.paint(i);
                i += 1;
                skip = *buffer.get_unchecked(i as usize);
                // Don't add a dependency from `skip` to the next `i` by adding
                // `skip` to `i` - it turns this loop into a pointer-chasing loop,
                // almost halving its throughput
                if skip != 0 {
                    // Only if we could simply jump into the above `if` block...
                    break;
                }
            }
        }

        *buffer.get_unchecked_mut(range.end as usize) = end_skip;
    }

    #[inline]
    fn paint_all<T: CovPainter>(&mut self, mut painter: T) {
        let buffer = &mut self.buffer[..];
        let mut i = 0;
        let mut skip = *unsafe { buffer.get_unchecked(i as usize) };

        loop {
            if skip != 0 {
                i += skip;
                if (i & EOB_BIT) != 0 {
                    return;
                }
                painter.skip(skip);
                skip = *unsafe { buffer.get_unchecked(i as usize) };
                continue;
            }

            loop {
                painter.paint(i);
                i += 1;

                skip = *unsafe { buffer.get_unchecked(i as usize) };
                // Don't add a dependency from `skip` to the next `i` by adding
                // `skip` to `i` - it turns this loop into a pointer-chasing loop,
                // almost halving its throughput
                if skip != 0 {
                    break;
                }
            }
        }
    }
}
