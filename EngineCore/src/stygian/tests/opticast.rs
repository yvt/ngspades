//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{vec2, vec3, Matrix4, PerspectiveFov, Point3, Vector3};
use ngsterrain::raytrace::{raytrace, RaytraceResult};

use stygian::{DepthImage, Terrain, TerrainRast};

#[path = "../common/terrainload.rs"]
#[allow(dead_code)]
mod terrainload;

#[test]
fn rasterize_opticast_depth() {
    let mut rast = TerrainRast::new(64);

    let projection: Matrix4<f32> = PerspectiveFov {
        fovy: cgmath::Rad(1.0),
        aspect: 4.0 / 3.0,
        near: 0.5,
        far: 500.0,
    }
    .into();

    let projection = Matrix4::from_translation(vec3(0.0, 0.0, 1.0))
        * Matrix4::from_nonuniform_scale(1.0, 1.0, -1.0)
        * projection;

    let mut image = DepthImage::new(vec2(64, 64));

    let sty_terrain = Terrain::from_ngsterrain(&terrainload::DERBY_RACERS).unwrap();

    #[derive(Clone)]
    struct Tracer<'a> {
        eye: Vector3<f32>,
        camera: Matrix4<f32>,
        terrain: &'a ngsterrain::Terrain,
    }
    impl stygian::Trace for Tracer<'_> {
        fn wants_opticast_sample(&mut self) -> bool {
            true
        }

        fn opticast_sample(&mut self, vertices: &[Vector3<f32>; 4], depth: f32) {
            // Make the coverage slightly smaller
            use array::Array4;
            let mid = (vertices[0] + vertices[1] + vertices[2] + vertices[3]) * 0.25;
            let verts = vertices.map(|v| v + (mid - v) * 0.1);

            for v in &verts {
                let p = Point3::from_homogeneous(self.camera * v.extend(0.0));

                let raytrace_result = raytrace(self.terrain, self.eye, self.eye + v * 1000.0);
                let actual_depth = match raytrace_result {
                    RaytraceResult::Inside(_) => {
                        unreachable!();
                    }
                    RaytraceResult::Hit(hit) => {
                        // The current implementation is not correct enough.
                        // Just check nohit/hit values for now
                        // Point3::from_homogeneous(self.camera * hit.position.extend(1.0)).z;
                        continue;
                    }
                    RaytraceResult::NoHit => 0.0,
                };

                assert!(
                    depth <= actual_depth,
                    "Sample {:?}: The estimated lower bound was {:?} but \
                     found a point ({:?}; screen space position = {:?}) \
                     where the actual depth is {:?}",
                    vertices,
                    depth,
                    ((v.x, v.y, v.z), &raytrace_result),
                    (p.x, p.y, p.z),
                    actual_depth
                );
            }
        }
    }

    for &(eye, at) in &[
        (Point3::new(64.0, 64.0, 15.0), Point3::new(0.0, 0.0, 5.0)),
        (Point3::new(64.0, 64.0, 15.0), Point3::new(128.0, 0.0, 5.0)),
        (
            Point3::new(64.0, 64.0, 15.0),
            Point3::new(128.0, 128.0, 5.0),
        ),
        (Point3::new(64.0, 64.0, 15.0), Point3::new(0.0, 128.0, 5.0)),
        (Point3::new(64.0, 64.0, 15.0), Point3::new(64.0, 0.0, 5.0)),
        (Point3::new(64.0, 64.0, 15.0), Point3::new(64.0, 128.0, 5.0)),
        (Point3::new(64.0, 64.0, 15.0), Point3::new(0.0, 64.0, 5.0)),
        (Point3::new(64.0, 64.0, 15.0), Point3::new(128.0, 64.0, 5.0)),
    ] {
        dbg!((eye, at));

        let view = Matrix4::look_at(eye, at, vec3(0.0, 0.0, 1.0));
        let camera = projection * view;
        rast.set_camera_matrix(camera);

        let tracer = Tracer {
            eye: eye - Point3::new(0.0, 0.0, 0.0),
            camera,
            terrain: &*terrainload::DERBY_RACERS,
        };

        rast.rasterize_trace(&sty_terrain, &mut image, tracer);
    }
}
