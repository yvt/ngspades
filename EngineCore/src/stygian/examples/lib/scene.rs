//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{conv::array4x4, prelude::*, vec2, Matrix4, Point3};
use glium::{backend::Facade, program, uniform, IndexBuffer, Program, Surface, VertexBuffer};
use ngsterrain;
use std::path::Path;

use crate::lib::{terrainload, vxl2mesh};

#[derive(Debug)]
pub struct Scene {
    ngs_terrain: ngsterrain::Terrain,
}

impl Scene {
    pub fn load(input_path: impl AsRef<Path>) -> Self {
        Self {
            ngs_terrain: terrainload::load_terrain(input_path),
        }
    }

    pub fn load_derby_racers() -> Self {
        Self {
            ngs_terrain: terrainload::DERBY_RACERS.clone(),
        }
    }

    pub fn make_sty_terrain(&self) -> (stygian::Terrain, Matrix4<f32>) {
        (
            stygian::Terrain::from_ngsterrain(&self.ngs_terrain).unwrap(),
            Matrix4::identity(),
        )
    }

    pub fn camera_initial_position(&self) -> Point3<f32> {
        let size = self.ngs_terrain.size();

        let eye_xy = vec2(size.x / 2, size.y / 2);
        let floor = (self.ngs_terrain.get_row(eye_xy).unwrap())
            .chunk_z_ranges()
            .last()
            .unwrap()
            .end;
        let eye_z = floor + size.z / 10 + 1;

        Point3::new(eye_xy.x as f32, eye_xy.y as f32, eye_z as f32)
    }
}

#[derive(Debug)]
pub struct SceneRenderer {
    program: Program,
}

impl SceneRenderer {
    pub fn new(facade: &impl Facade) -> Self {
        let program = program!(facade,
        100 => {
            vertex: r"
                #version 100

                uniform highp mat4 u_matrix;
                attribute highp vec3 pos;
                attribute highp vec3 norm;
                attribute highp vec4 color;
                varying lowp vec4 v_color;

                void main() {
                    v_color = color / 255.0;
                    v_color *= sqrt(dot(norm, normalize(vec3(0.3, 0.7, 0.8))) * 0.5 + 0.5);
                    gl_Position = u_matrix * vec4(pos, 1.0);

                    // Simulate [0, 1] Z range
                    gl_Position.z = gl_Position.z * 2.0 - gl_Position.w;
                }
            ",
            fragment: r"
                #version 100

                varying lowp vec4 v_color;

                void main() {
                    gl_FragColor = v_color;
                }
            ",
        })
        .unwrap();
        SceneRenderer { program }
    }
}

#[derive(Debug)]
pub struct SceneInstance {
    vb: VertexBuffer<vxl2mesh::TerrainVertex>,
    ib: IndexBuffer<u32>,
}

impl SceneRenderer {
    pub fn prepare_scene(&self, facade: &impl Facade, scene: &Scene) -> SceneInstance {
        // Convert the terrain to a mesh
        println!("Converting the terrain into a mesh");
        let (verts, indices) = vxl2mesh::terrain_to_mesh(&scene.ngs_terrain);
        let vb = VertexBuffer::new(facade, &verts).unwrap();
        let ib =
            IndexBuffer::new(facade, glium::index::PrimitiveType::TrianglesList, &indices).unwrap();

        SceneInstance { vb, ib }
    }

    pub fn draw_scene(
        &self,
        instance: &SceneInstance,
        target: &mut impl Surface,
        pvm_matrix: Matrix4<f32>,
    ) {
        let uniforms = uniform! {
            u_matrix: array4x4(pvm_matrix),
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfMore,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };

        target
            .draw(
                &instance.vb,
                &instance.ib,
                &self.program,
                &uniforms,
                &params,
            )
            .unwrap();
    }
}
