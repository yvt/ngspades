//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Vector3;

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

    /// Construct a `ColoredVoxel` with default properties.
    ///
    /// The color is generated deterministically. The material ID is always zero.
    pub fn default(position: Vector3<usize>) -> Self {
        let mut c = position.x as u32 ^ ((position.y as u32) << 8) ^ ((position.z as u32) << 16);

        // randomize
        c ^= c << 13;
        c ^= c >> 17;
        c ^= c << 5;
        c ^= c << 13;
        c ^= c >> 17;
        c ^= c << 5;

        Self::from_values([c as u8, (c >> 8) as u8, (c >> 16) as u8], 0)
    }
}

impl<T: AsRef<[u8]>> ColoredVoxel<T> {
    /// Get the color value of the voxel.
    pub fn color(&self) -> &[u8; 3] {
        array_ref![self.0.as_ref(), 0, 3]
    }

    /// Get the material ID of the voxel.
    pub fn material(&self) -> &u8 {
        &self.0.as_ref()[3]
    }

    /// Create a owned `ColoredVoxel` by cloning the underlying data.
    pub fn into_owned(&self) -> ColoredVoxel<[u8; 4]> {
        ColoredVoxel::new(array_ref![self.0.as_ref(), 0, 4].clone())
    }
}

impl<T, Rhs> PartialEq<ColoredVoxel<Rhs>> for ColoredVoxel<T>
where
    T: PartialEq<Rhs>,
{
    fn eq(&self, other: &ColoredVoxel<Rhs>) -> bool {
        self.0 == other.0
    }
}

impl<T: Eq> Eq for ColoredVoxel<T> {}

impl<T: AsMut<[u8]>> ColoredVoxel<T> {
    /// Get a mutable reference to the color value of the voxel.
    pub fn color_mut(&mut self) -> &mut [u8; 3] {
        array_mut_ref![self.0.as_mut(), 0, 3]
    }

    /// Get a mutable reference to the material ID of the voxel.
    pub fn material_mut(&mut self) -> &mut u8 {
        &mut self.0.as_mut()[3]
    }

    /// Copy values from another `ColoredVoxel` into this one.
    pub fn copy_from<U: AsRef<[u8]>>(&mut self, other: &ColoredVoxel<U>) {
        self.0.as_mut()[0..4].copy_from_slice(&other.0.as_ref()[0..4]);
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

impl<T: AsRef<[u8]>> SolidVoxel<T> {
    /// Create a owned `SolidVoxel` by cloning the underlying data.
    pub fn into_owned(&self) -> SolidVoxel<[u8; 4]> {
        match self {
            &SolidVoxel::Colored(ref cv) => SolidVoxel::Colored(cv.into_owned()),
            &SolidVoxel::Uncolored => SolidVoxel::Uncolored
        }
    }
}
