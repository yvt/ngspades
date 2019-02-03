/*
Copied and modified from:

    <https://github.com/PistonDevelopers/gfx_voxel/blob/master/src/cube.rs>

The MIT License (MIT)

Copyright (c) 2014 PistonDevelopers

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

//! Helper methods and structures for working with cubes.
//!
//! ```ignore
//!         3  ---------  2
//!           /       / |
//!          /  up   /  |
//!      6  -------- 7  | 1
//!        |        |  /
//! west   |  south | /  east
//!        |        |/
//!      5  -------- 4
//! ```
//!
//!
//! ```ignore
//!         7  ---------  6
//!           /       / |
//!          /  up   /  |
//!      2  -------- 3  | 5
//!        |        |  /
//! east   |  north | /  west
//!        |        |/
//!      1  -------- 0
//! ```

use cgmath::Vector3;
use std::str::FromStr;

pub use self::Face::{Down, East, North, South, Up, West};

/// Cube faces (clockwise).
pub const QUADS: &'static [[usize; 4]; 6] = &[
    [1, 0, 5, 4], // down
    [7, 6, 3, 2], // up
    [0, 1, 2, 3], // north
    [4, 5, 6, 7], // south
    [5, 0, 3, 6], // west
    [1, 4, 7, 2], // east
];

/// Cube vertices.
pub const VERTICES: &'static [[u8; 3]; 8] = &[
    // This is the north surface
    [0, 0, 0], // 0
    [1, 0, 0], // 1
    [1, 1, 0], // 2
    [0, 1, 0], // 3
    // This is the south surface
    [1, 0, 1], // 4
    [0, 0, 1], // 5
    [0, 1, 1], // 6
    [1, 1, 1], // 7
];

/// A value representing face direction.
#[repr(usize)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub enum Face {
    /// Facing down.
    Down,
    /// Facing up.
    Up,
    /// Facing north.
    North,
    /// Facing south.
    South,
    /// Facing west.
    West,
    /// Facing east.
    East,
}

impl Face {
    /// Computes vertices of the face.
    pub fn vertices(self) -> [Vector3<u8>; 4] {
        use array::*;

        QUADS[self as usize]
            .map(|i| VERTICES[i])
            .map(|v| Vector3::new(v[0], v[1], v[2]))
    }

    /// Gets the direction of face.
    pub fn direction(self) -> Vector3<i8> {
        match self {
            Down => Vector3::new(0, -1, 0),
            Up => Vector3::new(0, 1, 0),
            North => Vector3::new(0, 0, -1),
            South => Vector3::new(0, 0, 1),
            West => Vector3::new(-1, 0, 0),
            East => Vector3::new(1, 0, 0),
        }
    }

    /// Gets the face in a specific direction.
    pub fn from_direction(d: [i32; 3]) -> Option<Self> {
        Some(match (d[0], d[1], d[2]) {
            (0, -1, 0) => Down,
            (0, 1, 0) => Up,
            (0, 0, -1) => North,
            (0, 0, 1) => South,
            (-1, 0, 0) => West,
            (1, 0, 0) => East,
            _ => return None,
        })
    }

    /// Convert number to face.
    pub fn from_usize(number: usize) -> Option<Self> {
        Some(match number {
            0 => Down,
            1 => Up,
            2 => North,
            3 => South,
            4 => West,
            5 => East,
            _ => return None,
        })
    }
}

/// The error parsing face from string.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ParseError;

impl FromStr for Face {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        Ok(match s {
            "down" => Down,
            "up" => Up,
            "north" => North,
            "south" => South,
            "west" => West,
            "east" => East,
            _ => return Err(ParseError),
        })
    }
}

/// Iterates through each face on a cube.
#[derive(Copy, Clone)]
pub struct FaceIterator(usize);

impl FaceIterator {
    /// Creates a new face iterator.
    pub fn new() -> Self {
        FaceIterator(0)
    }
}

impl Iterator for FaceIterator {
    type Item = Face;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        let face = self.0;
        if face < 6 {
            self.0 += 1;
            Face::from_usize(face)
        } else {
            None
        }
    }
}
