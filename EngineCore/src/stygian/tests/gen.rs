//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use array::Array3;
use cgmath::{prelude::*, vec2, vec3, Matrix4, Point3};
use gltf;
use pod::Pod;
use xz_decom::decompress;

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

        let model_matrix =
            Matrix4::from_scale(7.0) * Matrix4::from_translation(vec3(17.0, 8.0, 0.0));

        let gltf_blob = decompress(include_bytes!("sponza.glb.xz")).unwrap();
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

    let _voxels = gen::VoxelBitmap::from_geometry(&pool, &domain, &binned_geometry);
}

fn vertex_fetch<T: Pod>(view: &gltf::buffer::View, blob: &[u8], i: usize) -> T {
    let size = std::mem::size_of::<T>();
    let stride = view.stride().unwrap_or(size);
    let start = view.offset() + stride * i;
    T::from_bytes(&blob[start..start + size]).unwrap()
}
