//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::borrow::Borrow;
use std::io::{Cursor, SeekFrom, Seek};
use byteorder::{LittleEndian, ReadBytesExt};

use {RowSolidVoxels, SolidVoxel};

mod chunkiter;
mod soliditer;
pub use self::chunkiter::*;
pub use self::soliditer::*;

/// NgsTF-encoded row data (an array of voxels that reside in the same X and Y
/// coordinate).
///
/// `T` is a type used to borrow a reference to `[u8]`. The borrowed `[u8]` must
/// be a valid raw NgsTF row data.
///
/// # NgsTF Row Data Specification
///
/// Each row data is a zero or more long sequence of "solid chunks" ordered by
/// their Z values.
///
/// ```text
/// <row> ::= { <solid-chunk> }
/// ```
///
/// Each solid chunk represents one or more densely packed solid voxels, with
/// optional color and material information. Zero or more empty voxels might
/// precede.
///
/// ```text
/// <solid-chunk> ::= <empty-voxels> <colored-voxels>
///                   { <uncolored-voxels> <colored-voxels> }
///                   %x00 %x00
///
/// <empty-voxels> ::= U16
///
/// <uncolored-voxels> ::= U16
///
/// <colored-voxels> ::= U16 { <color> }
/// ```
///
/// Each integer (`U16`) in `<empty-voxels>`, `<uncolored-voxels>`, and
/// `<colored-voxels>` specifies the number of voxels in a row. None of them can
/// be zero as it would be parsed as a terminator of `<solid-chunk>`.
///
/// `<colored-voxels>` contains a color/material information (`<color>`) for
/// every voxel in it. `<color>` is composed of four `U8`s representing the red,
/// green, blue component of the color, and the material ID, respectively.
/// See the documentation of [`ColoredVoxel`] for the definition of `<color>`.
///
/// [`ColoredVoxel`]: struct.ColoredVoxel.html
///
#[derive(Debug, Clone)]
pub struct Row<T>(usize, T);

impl<T> Row<T> {
    /// Constructs a `Row`.
    pub fn new(depth: usize, data: T) -> Self {
        Row(depth, data)
    }

    /// Get the depth (size in the Z axis direction) of the row.
    pub fn depth(&self) -> usize {
        self.0
    }

    /// Get a reference to the underlying storage.
    pub fn get_inner_ref(&self) -> &T {
        &self.1
    }

    /// Get a mutable reference to the underlying storage.
    pub fn get_inner_ref_mut(&mut self) -> &mut T {
        &mut self.1
    }

    /// Unwrap this `Row`, returning the underlying storage.
    pub fn into_inner(self) -> T {
        self.1
    }

    /// Convert from `Row<T>` to `Row<&T>`.
    pub fn as_ref(&self) -> Row<&T> {
        Row(self.0, &self.1)
    }
}

pub type RowValidationError = &'static str;

impl<'a, T: Borrow<[u8]>> Row<&'a T> {
    /// Retrieve `SolidVoxel` at the specified location in the row.
    pub fn get_voxel(&self, z: usize) -> Option<SolidVoxel<&'a [u8; 4]>> {
        if z >= self.0 {
            None
        } else {
            let mut chunks = self.chunks();
            while let Some(chunk) = chunks.next() {
                for (voxels_z, voxels) in chunk {
                    if voxels_z > z {
                        return None;
                    } else if voxels_z + voxels.num_voxels() > z {
                        return match voxels {
                            RowSolidVoxels::Colored(voxels) => Some(SolidVoxel::Colored(
                                voxels.get(z - voxels_z).unwrap(),
                            )),
                            RowSolidVoxels::Uncolored(_) => Some(SolidVoxel::Uncolored),
                        };
                    }
                }
            }
            None
        }
    }

    /// Validate the conformity of the row format.
    pub fn validate(&self) -> Result<(), RowValidationError> {
        let mut cursor = Cursor::new(self.1.borrow());
        let mut z = 0usize;
        let depth = self.0;

        while (cursor.position() as usize) < cursor.get_ref().len() {
            // Check this chunk
            // <empty-voxels>
            let num_voxels = cursor.read_u16::<LittleEndian>().or(Err(
                "Encountered EOF while reading <empty-voxels>",
            ))?;
            z += num_voxels as usize;
            if z > depth {
                return Err("Z value overflow");
            }

            // First <colored-voxels>
            let num_voxels = cursor.read_u16::<LittleEndian>().or(Err(
                "Encountered EOF while reading U16 of <colored-voxels>",
            ))?;
            if num_voxels == 0 {
                return Err("U16 of <colored-voxels> is zero");
            }
            z += num_voxels as usize;
            if z > depth {
                return Err("Z value overflow");
            }

            // Skip color values
            cursor
                .seek(SeekFrom::Current(num_voxels as i64 * 4))
                .or(Err(
                    "Encountered EOF while reading color values of <colored-voxels>",
                ))?;

            loop {
                // <uncolored-voxels>
                let num_voxels = cursor.read_u16::<LittleEndian>().or(Err(
                    "Encountered EOF while reading <uncolored-voxels>",
                ))?;
                if num_voxels == 0 {
                    break;
                }
                z += num_voxels as usize;
                if z > depth {
                    return Err("Z value overflow");
                }

                // <colored-voxels>
                let num_voxels = cursor.read_u16::<LittleEndian>().or(Err(
                    "Encountered EOF while reading U16 of <colored-voxels>",
                ))?;
                if num_voxels == 0 {
                    return Err("U16 of <colored-voxels> is zero");
                }
                z += num_voxels as usize;
                if z > depth {
                    return Err("Z value overflow");
                }

                // Skip color values
                cursor
                    .seek(SeekFrom::Current(num_voxels as i64 * 4))
                    .or(Err(
                        "Encountered EOF while reading color values of <colored-voxels>",
                    ))?;
            }
        }
        Ok(())
    }
}
