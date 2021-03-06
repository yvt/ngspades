//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use array::Array3;
use cgmath::{prelude::*, vec2, vec3, Matrix4, Point3};
use gltf;
use pod::Pod;
use std::io::{Cursor, prelude::*};
use xz2::read::XzDecoder;

use stygian::{gen, mempool};

#[path = "../common/profmempool.rs"]
mod profmempool;

#[test]
fn gen_from_gltf() {
    let domain = gen::InitialDomain {
        tile_size_bits: 5, // 2⁵ == 32
        tile_count: vec2(4, 4),
        depth: 32,
    };
    dbg!(domain);

    let pool = profmempool::ProfMemPool::new(mempool::SysMemPool);
    let mut binned_geometry = gen::BinnedGeometry::new(&pool, &domain);

    {
        let mut binner = gen::PolygonBinner::new(256, &domain, &mut binned_geometry);

        let model_matrix = Matrix4::from_scale(3.5)
            * Matrix4::from_translation(vec3(17.0, 8.0, 0.0))
            * Matrix4::from_angle_x(cgmath::Deg(90.0));

        let mut decoder = XzDecoder::new(Cursor::new(&include_bytes!("sponza.glb.xz")[..]));
        let mut gltf_blob = Vec::new();
        decoder.read_to_end(&mut gltf_blob).unwrap();
        let gltf = gltf::Gltf::from_slice(&gltf_blob).unwrap();
        let blob = gltf.blob.as_ref().unwrap();
        for mesh in gltf.meshes() {
            println!("found mesh {:?}", mesh.name());
            for prim in mesh.primitives() {
                let positions = prim.get(&gltf::Semantic::Positions).unwrap();
                let indices = prim.indices().unwrap();

                assert_eq!(positions.size(), 12);
                assert_eq!(positions.data_type(), gltf::accessor::DataType::F32);

                assert_eq!(indices.data_type(), gltf::accessor::DataType::U16);

                for i in (0..indices.count()).step_by(3) {
                    let idx: [u16; 3] = [
                        vertex_fetch(&indices.view(), blob, i),
                        vertex_fetch(&indices.view(), blob, i + 1),
                        vertex_fetch(&indices.view(), blob, i + 2),
                    ];
                    let positions: [Point3<f32>; 3] = idx
                        .map(|i| vertex_fetch::<[f32; 3]>(&positions.view(), blob, i as usize))
                        .map(|v| v.into());

                    binner.insert(positions.map(|v| model_matrix.transform_point(v)));
                }
            }
        }

        binner.flush();
    }

    println!("from_geometry");
    let mut voxels = gen::VoxelBitmap::from_geometry(&pool, &domain, &binned_geometry);
    // dbg!(&voxels);

    println!("flood_fill_in_place");
    voxels.flood_fill_in_place(
        &domain,
        &[Point3::new(64, 32, 16)],
        gen::VoxelType::Empty,
        gen::VoxelType::View,
    );
    // dbg!(&voxels);

    println!("erode_view");
    let voxels2 = voxels.erode_view(&pool, &domain);
    dbg!(&voxels2);

    println!("to_terrain");
    let _terrain = voxels2.to_terrain(&domain, 2);
}

fn vertex_fetch<T: Pod>(view: &gltf::buffer::View, blob: &[u8], i: usize) -> T {
    let size = std::mem::size_of::<T>();
    let stride = view.stride().unwrap_or(size);
    let start = view.offset() + stride * i;
    T::from_bytes(&blob[start..start + size]).unwrap()
}
