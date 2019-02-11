//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Contains implementations of a coverage buffer, which is used to implement
//! the reverse painter's algorithm.
use std::ops::Range;

/// A coverage buffer.
///
/// Conceptually, a coverage buffer contains zero or more boolean values each
/// representing the corresponding pixel has already been painted.
///
/// `CovBuffer` is marked as `unsafe` because of the following reasons:
///  - `paint` must not advance the cursor past the boundary.
///  - The provided implementation of `paint_all` depends on the return value
///    of `len` to call `paint`.
pub unsafe trait CovBuffer {
    /// Expand the backing store of the coverage buffer to contain at least
    /// `size` elements.
    fn reserve(&mut self, _size: u32) {}

    /// Reset and resize the coverage buffer.
    fn resize(&mut self, size: u32);

    /// Get the number of elements.
    fn len(&self) -> u32;

    /// Paint a range of pixels.
    ///
    /// The clients should maintain a cursor position starting at `range.start`.
    /// `CovBuffer` calls one of `painter`'s methods for each span of
    /// painted/unpainted pixels found at the cursor position.
    /// Every time one of `painter`'s methods is called, the cursor is advanced
    /// by a specified number of pixels.
    ///
    /// *Unsafety: `range` must be in the bounds specified by `resize`.
    unsafe fn paint<T: CovPainter>(&mut self, range: Range<u32>, painter: T);

    /// Paint all unpainted pixels.
    ///
    /// It's allowed to assume that this is the last painting operation before
    /// `resize` is called for the next time.
    fn paint_all<T: CovPainter>(&mut self, painter: T) {
        unsafe { self.paint(0..self.len(), painter) }
    }
}

/// The callback methods for [`CovBuffer::paint`].
pub trait CovPainter {
    /// Advance the cursor by a specified number of pixels without updating
    /// the corresponding pixels.
    fn skip(&mut self, _count: u32) {}

    /// Paint a pixel at the current cursor position (`i`). The cursor is
    /// advanced by one pixel.
    fn paint(&mut self, _i: u32) {}
}

mod bitarray;
mod skip;
pub use self::bitarray::*;
pub use self::skip::*;
