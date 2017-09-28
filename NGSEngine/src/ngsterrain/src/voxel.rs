//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::borrow::{Borrow, BorrowMut};

/// A solid voxel with color/material data.
///
/// The underlying storage of the type `T` is `[u8]` with four elements, or a
/// reference to it.
///
/// # NgsTF Voxel Data Specification
///
/// `<color>` is composed of four `U8`s representing the red, green, blue
/// component of the color, and the material ID, respectively.
///
/// ```text
/// <color> ::= U8 U8 U8 U8
/// ```
///
/// The color is specified in the [non-linear sRGB color space]. The semantics of
/// material ID is not defined by this specification.
///
/// [non-linear sRGB color space]: https://en.wikipedia.org/wiki/SRGB
#[derive(Debug, Clone, Copy)]
pub struct ColoredVoxel<T>(T);

impl<T> ColoredVoxel<T> {
    /// Construct a `ColoredVoxel`.
    pub fn new(data: T) -> Self {
        ColoredVoxel(data)
    }

    /// Get a reference to the underlying storage.
    pub fn get_inner_ref(&self) -> &T {
        &self.0
    }

    /// Get a mutable reference to the underlying storage.
    pub fn get_inner_ref_mut(&mut self) -> &mut T {
        &mut self.0
    }

    /// Unwrap this `ColoredVoxel`, returning the underlying storage.
    pub fn into_inner(self) -> T {
        self.0
    }

    /// Convert from `ColoredVoxel<T>` to `ColoredVoxel<&T>`.
    pub fn as_ref(&self) -> ColoredVoxel<&T> {
        ColoredVoxel(&self.0)
    }
}

impl ColoredVoxel<[u8; 4]> {
    /// Construct a `ColoredVoxel` with given properties.
    pub fn from_values(color: [u8; 3], material: u8) -> Self {
        Self::new([color[0], color[1], color[2], material])
    }
}

impl<T: Borrow<[u8]>> ColoredVoxel<T> {
    /// Get the color value of the voxel.
    pub fn color(&self) -> &[u8; 3] {
        array_ref![self.0.borrow(), 0, 3]
    }

    /// Get the material ID of the voxel.
    pub fn material(&self) -> &u8 {
        &self.0.borrow()[3]
    }

    /// Create a owned `ColoredVoxel` by cloning the underlying data.
    pub fn into_owned(&self) -> ColoredVoxel<[u8; 4]> {
        ColoredVoxel::new(array_ref![self.0.borrow(), 0, 4].clone())
    }
}

impl<T, Rhs> PartialEq<ColoredVoxel<Rhs>> for ColoredVoxel<T>
where
    T: PartialEq<Rhs>,
{
    fn eq(&self, other: &ColoredVoxel<Rhs>) -> bool {
        self.0.borrow() == other.0.borrow()
    }
}

impl<T: Eq> Eq for ColoredVoxel<T> {}

impl<T: BorrowMut<[u8]>> ColoredVoxel<T> {
    /// Get a mutable reference to the color value of the voxel.
    pub fn color_mut(&mut self) -> &mut [u8; 3] {
        array_mut_ref![self.0.borrow_mut(), 0, 3]
    }

    /// Get a mutable reference to the material ID of the voxel.
    pub fn material_mut(&mut self) -> &mut u8 {
        &mut self.0.borrow_mut()[3]
    }

    /// Copy values from another `ColoredVoxel` into this one.
    pub fn copy_from<U: Borrow<[u8]>>(&mut self, other: &ColoredVoxel<U>) {
        self.0.borrow_mut()[0..4].copy_from_slice(&other.0.borrow()[0..4]);
    }
}

/// A solid voxel with or without color/material data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidVoxel<T> {
    /// A solid voxel with color/material data.
    Colored(ColoredVoxel<T>),

    /// A solid voxel without color/material data.
    Uncolored,
}
