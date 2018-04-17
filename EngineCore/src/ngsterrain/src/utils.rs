//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/// Load a `u16` from a given slice.
///
/// The given `index` must be a multiple of `2`.
#[inline(always)]
pub unsafe fn load_u16_le(data: &[u8], index: usize) -> u16 {
    assert!(index + 1 < data.len(), "out of bounds");
    debug_assert!(index % 2 == 0);

    let p = data.as_ptr().offset(index as isize);
    <u16>::from_le(*(p as *const _))
}
