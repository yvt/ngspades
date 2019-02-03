//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use bencher::{Bencher, benchmark_group, benchmark_main};
use cgmath::{Matrix4, Perspective, Point3, vec3};

use stygian::TerrainRast;

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

benchmark_group!(benches, set_camera_matrix);
benchmark_main!(benches);
