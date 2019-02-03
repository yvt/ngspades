//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{vec2, Vector3};
use glium::implement_vertex;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TerrainVertex {
    pos: [i16; 3],
    _pad1: i16,
    norm: [i16; 3],
    _pad2: i16,
    color: [u8; 4],
    _pad3: u32,
}

implement_vertex!(TerrainVertex, pos, norm, color);

pub fn terrain_to_mesh(terrain: &ngsterrain::Terrain) -> (Vec<TerrainVertex>, Vec<u32>) {
    use super::cube::Face;

    let mut verts = Vec::new();
    let mut indices = Vec::new();

    let depth = terrain.size().z;

    {
        let mut push_face = |face: Face, origin: Vector3<usize>, color: [u8; 3]| {
            let origin = origin.cast::<i16>().unwrap();
            let face_verts = face.vertices();

            verts.reserve(4);
            indices.reserve(6);

            let start = verts.len() as u32;
            for v in &face_verts {
                let p = v.cast::<i16>().unwrap() + origin;
                verts.push(TerrainVertex {
                    pos: [p.x, p.y, p.z],
                    norm: face.direction().cast::<i16>().unwrap().into(),
                    color: [color[0], color[1], color[2], 255],
                    ..Default::default()
                });
            }

            indices.push(start);
            indices.push(start + 1);
            indices.push(start + 3);
            indices.push(start + 1);
            indices.push(start + 2);
            indices.push(start + 3);
        };

        for (xy, row) in terrain.rows() {
            let row = row_chunk_iter_to_voxels(row.chunks(), depth);

            for &face in &[Face::Up, Face::Down, Face::West, Face::East] {
                let dir = face.direction();
                let row2 = terrain
                    .get_row(vec2(
                        xy.x.wrapping_add(dir.x as usize),
                        xy.y.wrapping_add(dir.y as usize),
                    ))
                    .map(|row| row_chunk_iter_to_voxels(row.chunks(), depth));

                for i in 0..depth {
                    if let (Some(v), None) = (row[i], row2.as_ref().and_then(|r| r[i])) {
                        push_face(face, xy.extend(i), color_for_solid_voxel(v));
                    }
                }
            }

            for i in 1..depth {
                if let (Some(v), None) = (row[i], row[i - 1]) {
                    push_face(Face::North, xy.extend(i), color_for_solid_voxel(v));
                }
            }
            for i in 0..depth - 1 {
                if let (Some(v), None) = (row[i], row[i + 1]) {
                    push_face(Face::South, xy.extend(i), color_for_solid_voxel(v));
                }
            }
        }
    }

    (verts, indices)
}

fn color_for_solid_voxel(voxel: ngsterrain::SolidVoxel<&[u8; 4]>) -> [u8; 3] {
    use ngsterrain::SolidVoxel;
    match voxel {
        SolidVoxel::Colored(x) => *x.color(),
        SolidVoxel::Uncolored => [128, 128, 128],
    }
}

fn row_chunk_iter_to_voxels(
    mut iter: ngsterrain::RowChunkIter<&[u8]>,
    depth: usize,
) -> Vec<Option<ngsterrain::SolidVoxel<&[u8; 4]>>> {
    use ngsterrain::{RowSolidVoxels, SolidVoxel};

    let mut out = vec![None; depth];
    while let Some(chunk) = iter.next() {
        for (voxels_z, voxels) in chunk {
            match voxels {
                RowSolidVoxels::Colored(voxels) => {
                    for i in 0..voxels.num_voxels() {
                        out[i + voxels_z] = Some(SolidVoxel::Colored(voxels.get(i).unwrap()));
                    }
                }
                RowSolidVoxels::Uncolored(num) => {
                    for x in out[voxels_z..voxels_z + num].iter_mut() {
                        *x = Some(SolidVoxel::Uncolored);
                    }
                }
            }
        }
    }

    out
}
