//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::borrow::Borrow;
use ColoredVoxel;

/// Consecutive solid voxels with or without color/material data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowSolidVoxels<T> {
    /// Consecutive solid voxels with color/material data as indicated by the
    /// contained `ColoredVoxels`.
    Colored(ColoredVoxels<T>),

    /// Consecutive solid voxels without color/material data. The contained
    /// number indicates the number of voxels.
    Uncolored(usize),
}

/// Consecutive solid voxels with color/material data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColoredVoxels<T>(T);

impl<T> ColoredVoxels<T> {
    /// Construct a `ColoredVoxels`.
    pub fn new(data: T) -> Self {
        ColoredVoxels(data)
    }
}

impl<T: Borrow<[u8]>> RowSolidVoxels<T> {
    /// Get the number of contained voxels.
    ///
    ///  - For `Colored(cv)`, it returns `cv.num_voxels()`.
    ///  - For `Uncolored(n)`, it returns `n`.
    ///
    pub fn num_voxels(&self) -> usize {
        match self {
            &RowSolidVoxels::Colored(ref cv) => cv.num_voxels(),
            &RowSolidVoxels::Uncolored(nv) => nv,
        }
    }
}

impl<T: Borrow<[u8]>> ColoredVoxels<T> {
    /// Get the number of contained voxels.
    pub fn num_voxels(&self) -> usize {
        debug_assert!(self.0.borrow().len() % 4 == 0);
        self.0.borrow().len() / 4
    }
}

impl<'a> ColoredVoxels<&'a [u8]> {
    #[allow(unused_unsafe)] // originates from `array_ref!`
    pub unsafe fn get_unchecked(&self, index: usize) -> ColoredVoxel<&'a [u8; 4]> {
        let slice = self.0.get_unchecked(index * 4..index * 4 + 4);
        ColoredVoxel::new(array_ref![slice, 0, 4])
    }

    pub fn get(&self, index: usize) -> Option<ColoredVoxel<&'a [u8; 4]>> {
        self.0.get(index * 4..index * 4 + 4).map(|array| {
            ColoredVoxel::new(array_ref![array, 0, 4])
        })
    }
}

impl<'a> ColoredVoxels<&'a mut [u8]> {
    #[allow(unused_unsafe)] // originates from `array_ref!`
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> ColoredVoxel<&mut [u8; 4]> {
        let slice = self.0.get_unchecked_mut(index * 4..index * 4 + 4);
        ColoredVoxel::new(array_mut_ref![slice, 0, 4])
    }

    pub fn get_mut(&mut self, index: usize) -> Option<ColoredVoxel<&mut [u8; 4]>> {
        self.0.get_mut(index * 4..index * 4 + 4).map(|array| {
            ColoredVoxel::new(array_mut_ref![array, 0, 4])
        })
    }

    #[allow(unused_unsafe)] // originates from `array_ref!`
    pub unsafe fn take_unchecked_mut(self, index: usize) -> ColoredVoxel<&'a mut [u8; 4]> {
        let slice = self.0.get_unchecked_mut(index * 4..index * 4 + 4);
        ColoredVoxel::new(array_mut_ref![slice, 0, 4])
    }

    pub fn take_mut(self, index: usize) -> Option<ColoredVoxel<&'a mut [u8; 4]>> {
        self.0.get_mut(index * 4..index * 4 + 4).map(|array| {
            ColoredVoxel::new(array_mut_ref![array, 0, 4])
        })
    }
}
