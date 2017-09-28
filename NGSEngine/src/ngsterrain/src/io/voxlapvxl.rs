//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::io::{Read, Result};
use cgmath::{Vector2, Vector3};

use {Terrain, SolidVoxel, ColoredVoxel};

/// Load a `Terrain` from a [Voxlap] VXL-encoded data.
///
/// [Voxlap](http://advsys.net/ken/voxlap.htm)
pub fn from_voxlap_vxl<T: Read>(size: Vector3<usize>, reader: &mut T) -> Result<Terrain> {
    let mut t = Terrain::new(size);
    let mut row_data = vec![None; size.z];
    let mut bottom_buf = Vec::with_capacity(size.z);

    fn read_color<T: Read>(reader: &mut T) -> Result<ColoredVoxel<[u8; 4]>> {
        let mut buf = [0; 4];
        reader.read_exact(&mut buf)?;
        Ok(ColoredVoxel::from_values([buf[0], buf[1], buf[2]], 0))
    }

    for y in 0..size.y {
        for x in 0..size.x {
            for x in row_data.iter_mut() {
                *x = Some(SolidVoxel::Uncolored);
            }

            bottom_buf.clear();

            loop {
                let mut buf = [0; 4];
                reader.read_exact(&mut buf)?;
                let num_4byte_chunks = buf[0];
                let top_color_start = buf[1];
                let top_color_end = buf[2];
                let air_start = buf[3];

                let mut z = air_start as usize - bottom_buf.len();
                for &x in bottom_buf.iter() {
                    row_data[z] = Some(SolidVoxel::Colored(x));
                    z += 1;
                }

                while z < top_color_start as usize {
                    row_data[z] = None;
                    z += 1;
                }

                while z <= top_color_end as usize {
                    row_data[z] = Some(SolidVoxel::Colored(read_color(reader)?));
                    z += 1;
                }

                if num_4byte_chunks == 0 {
                    break;
                }

                let len_bottom = num_4byte_chunks - (top_color_end + 1 - top_color_start) - 1;
                bottom_buf.clear();
                for _ in 0..len_bottom {
                    bottom_buf.push(read_color(reader)?);
                }
            }

            // The bottom must be capped with a colored voxel
            let mut last_color = row_data[0];
            for i in 1..row_data.len() {
                if row_data[i - 1] == Some(SolidVoxel::Uncolored) && row_data[i].is_none() {
                    row_data[i - 1] = last_color;
                }
                if let Some(SolidVoxel::Colored(_)) = row_data[i] {
                    last_color = row_data[i];
                }
            }
            if *row_data.last().unwrap() == Some(SolidVoxel::Uncolored) {
                *row_data.last_mut().unwrap() = last_color;
            }

            // Flip in the Z direction to match our coordinate system
            t.get_row_mut(Vector2::new(x, y))
                .unwrap()
                .update_with(row_data.iter().rev().map(Clone::clone))
                .unwrap();
        }
    }
    Ok(t)
}
