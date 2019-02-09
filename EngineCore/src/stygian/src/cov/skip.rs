//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Skip buffer.
use std::ops::Range;

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
        let end_skip = *buffer.get_unchecked(range.end as usize);

        while (i & EOB_BIT) == 0 {
            let skip = *buffer.get_unchecked(i as usize);
            if skip != 0 {
                i += skip;
                if i >= range.end {
                    return;
                }
                painter.skip(skip);
                continue;
            }

            loop {
                *buffer.get_unchecked_mut(i as usize) = end_skip + (range.end - i);
                painter.paint(i);
                i += 1;
                if i == range.end {
                    return;
                }
                let skip = *buffer.get_unchecked(i as usize);
                if skip != 0 {
                    break;
                }
            }
        }
    }

    #[inline]
    fn paint_all<T: CovPainter>(&mut self, mut painter: T) {
        let buffer = &mut self.buffer[..];
        let mut i = 0;

        while (i & EOB_BIT) == 0 {
            let skip = *unsafe { buffer.get_unchecked(i as usize) };
            if skip != 0 {
                painter.skip(skip);
                i += skip;
                continue;
            }

            loop {
                painter.paint(i);
                i += 1;

                let skip = *unsafe { buffer.get_unchecked(i as usize) };
                if skip != 0 {
                    painter.skip(skip);
                    i += skip;
                    break;
                }
            }
        }
    }
}
