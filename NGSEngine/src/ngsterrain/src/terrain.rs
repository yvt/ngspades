//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{Vector3, Vector2};
use cgmath::prelude::*;
use {Row, SolidVoxel, RowValidationError};

/// NgsTF-encoded terrain.
#[derive(Debug, Clone)]
pub struct Terrain {
    size: Vector3<usize>,

    /// Rows, stored in the X-major order.
    rows: Vec<Vec<u8>>,
}

impl Terrain {
    /// Constructs an empty `Terrain`.
    ///
    /// - `size.x` and `size.y` must be in the range `[1, 65535]`.
    /// - `size.z` must be in the range `[1, 1023]`.
    pub fn new(size: Vector3<usize>) -> Self {
        assert!(size.x >= 1 && size.x <= 65535, "size.x out of range");
        assert!(size.y >= 1 && size.y <= 65535, "size.y out of range");
        assert!(size.z >= 1 && size.z <= 1023, "size.z out of range");
        size.x.checked_mul(size.y).unwrap();

        Self {
            size,
            rows: vec![vec![]; size.x.checked_mul(size.y).unwrap()],
        }
    }

    pub fn size(&self) -> Vector3<usize> {
        self.size
    }

    fn index_for_row_unchecked(&self, pos: Vector2<usize>) -> usize {
        debug_assert!(pos.x < self.size.x);
        debug_assert!(pos.y < self.size.y);

        pos.x + pos.y * self.size.y
    }

    pub unsafe fn get_row_unchecked(&self, pos: Vector2<usize>) -> Row<&Vec<u8>> {
        let index = self.index_for_row_unchecked(pos);
        Row::new(self.size.z, self.rows.get_unchecked(index))
    }

    pub unsafe fn get_row_unchecked_mut(&mut self, pos: Vector2<usize>) -> Row<&mut Vec<u8>> {
        let index = self.index_for_row_unchecked(pos);
        Row::new(self.size.z, self.rows.get_unchecked_mut(index))
    }

    pub fn get_row(&self, pos: Vector2<usize>) -> Option<Row<&Vec<u8>>> {
        if pos.x < self.size.x && pos.y < self.size.y {
            Some(unsafe { self.get_row_unchecked(pos) })
        } else {
            None
        }
    }

    pub fn get_row_mut(&mut self, pos: Vector2<usize>) -> Option<Row<&mut Vec<u8>>> {
        if pos.x < self.size.x && pos.y < self.size.y {
            Some(unsafe { self.get_row_unchecked_mut(pos) })
        } else {
            None
        }
    }

    pub fn get_voxel(&self, pos: Vector3<usize>) -> Option<SolidVoxel<&[u8; 4]>> {
        self.get_row(pos.truncate()).and_then(
            |row| row.get_voxel(pos.z).unwrap_or(None),
        )
    }

    /// Get an iterator over the rows of the `Terrain`.
    ///
    /// The enumeration order is not specified.
    pub fn rows(&self) -> TerrainRows {
        TerrainRows {
            terrain: self,
            pos: Vector2::zero(),
            left: self.size.x * self.size.y,
        }
    }

    pub fn validate(&self) -> Result<(), (Vector2<usize>, RowValidationError)> {
        for (coord, row) in self.rows() {
            row.validate().map_err(|e| (coord, e))?;
        }
        Ok(())
    }
}

/// An iterator over the rows of a `Terrain`.
///
/// The enumeration order is not specified.
#[derive(Debug, Clone, Copy)]
pub struct TerrainRows<'a> {
    terrain: &'a Terrain,
    pos: Vector2<usize>,
    left: usize,
}

impl<'a> Iterator for TerrainRows<'a> {
    type Item = (Vector2<usize>, Row<&'a Vec<u8>>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.left > 0 {
            let row = (
                self.pos,
                unsafe { self.terrain.get_row_unchecked(self.pos) },
            );
            self.pos.x += 1;
            if self.pos.x == self.terrain.size.x {
                self.pos.x = 0;
                self.pos.y += 1;
            }
            self.left -= 1;
            Some(row)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.left, Some(self.left))
    }
}

impl<'a> ExactSizeIterator for TerrainRows<'a> {
    fn len(&self) -> usize {
        self.left
    }
}
