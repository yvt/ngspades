//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use bencher::{benchmark_group, benchmark_main, Bencher};
use cgmath::{vec2, vec3, Matrix4, Perspective, Point3};

use stygian::{DepthImage, Terrain, TerrainRast};

#[path = "../common/terrainload.rs"]
#[allow(dead_code)]
mod terrainload;

fn set_camera_matrix(b: &mut Bencher) {
    let mut rast = TerrainRast::new(512);

    let eye = Point3::new(1.0, 2.0, -3.0);
    let p: Matrix4<f32> = Perspective {
        left: -0.5,
        right: 0.5,
        top: 0.5,
        bottom: -0.5,
        near: 1.0,
        far: 100.0,
    }
    .into();
    let v = Matrix4::look_at(eye, Point3::new(40.0, -20.0, 30.0), vec3(0.2, 0.5, 0.8));

    b.iter(|| {
        rast.set_camera_matrix(p * v);
    });
}

fn rasterize(b: &mut Bencher, size: usize) {
    let mut rast = TerrainRast::new(size);

    let projection: Matrix4<f32> = Perspective {
        left: -0.5,
        right: 0.5,
        top: 0.5,
        bottom: -0.5,
        near: 1.0,
        far: 100.0,
    }
    .into();

    let projection = Matrix4::from_translation(vec3(0.0, 0.0, 1.0))
        * Matrix4::from_nonuniform_scale(1.0, 1.0, -1.0)
        * projection;

    let view = Matrix4::look_at(
        Point3::new(64.0, 64.0, 15.0),
        Point3::new(0.0, 0.0, 5.0),
        vec3(0.0, 0.0, 1.0),
    );
    rast.set_camera_matrix(projection * view);

    let mut image = DepthImage::new(vec2(size, size));

    let sty_terrain = Terrain::from_ngsterrain(&terrainload::DERBY_RACERS).unwrap();

    b.iter(|| {
        rast.rasterize(&sty_terrain, &mut image);
    });
}

fn rasterize_64(b: &mut Bencher) {
    rasterize(b, 64);
}
fn rasterize_256(b: &mut Bencher) {
    rasterize(b, 256);
}
fn rasterize_1024(b: &mut Bencher) {
    rasterize(b, 1024);
}

benchmark_group!(
    benches,
    set_camera_matrix,
    rasterize_64,
    rasterize_256,
    rasterize_1024
);
benchmark_main!(benches);
