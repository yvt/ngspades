//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{conv::array4x4, prelude::*, vec2, Matrix4, Point3};
use glium::{backend::Facade, program, uniform, IndexBuffer, Program, Surface, VertexBuffer};
use gltf;
use ngsterrain;
use std::{ffi::OsStr, fs::File, io::Read, path::Path};
use xz_decom::decompress;

use crate::lib::{terrainload, vxl2mesh};

#[derive(Debug)]
pub enum Scene {
    NgsTerrain { ngs_terrain: ngsterrain::Terrain },
    Gltf { gltf: gltf::Gltf },
}

impl Scene {
    pub fn load(input_path: impl AsRef<Path>) -> Self {
        let input_path: &Path = input_path.as_ref();
        // Assume `.xz` is a XZ-compressed glTF
        if input_path.extension().map(OsStr::to_str) == Some(Some("xz")) {
            let mut file = File::open(input_path).unwrap();
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).unwrap();

            let gltf_blob = decompress(&buffer).unwrap();
            let gltf = gltf::Gltf::from_slice(&gltf_blob).unwrap();
            Scene::Gltf { gltf }
        } else {
            // Otherwise, it's `.vox` or `.vxl`
            Scene::NgsTerrain {
                ngs_terrain: terrainload::load_terrain(input_path),
            }
        }
    }

    pub fn load_derby_racers() -> Self {
        Scene::NgsTerrain {
            ngs_terrain: terrainload::DERBY_RACERS.clone(),
        }
    }

    pub fn make_sty_terrain(&self) -> (stygian::Terrain, Matrix4<f32>) {
        match self {
            Scene::NgsTerrain { ngs_terrain } => (
                stygian::Terrain::from_ngsterrain(ngs_terrain).unwrap(),
                Matrix4::identity(),
            ),
            Scene::Gltf { gltf } => unimplemented!(),
        }
    }

    pub fn camera_initial_position(&self) -> Point3<f32> {
        match self {
            Scene::NgsTerrain { ngs_terrain } => {
                let size = ngs_terrain.size();

                let eye_xy = vec2(size.x / 2, size.y / 2);
                let floor = (ngs_terrain.get_row(eye_xy).unwrap())
                    .chunk_z_ranges()
                    .last()
                    .unwrap()
                    .end;
                let eye_z = floor + size.z / 10 + 1;

                Point3::new(eye_xy.x as f32, eye_xy.y as f32, eye_z as f32)
            }
            Scene::Gltf { gltf } => unimplemented!(),
        }
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
        match scene {
            Scene::NgsTerrain { ngs_terrain } => {
                // Convert the terrain to a mesh
                println!("Converting the terrain into a mesh");
                let (verts, indices) = vxl2mesh::terrain_to_mesh(ngs_terrain);
                let vb = VertexBuffer::new(facade, &verts).unwrap();
                let ib =
                    IndexBuffer::new(facade, glium::index::PrimitiveType::TrianglesList, &indices)
                        .unwrap();

                SceneInstance { vb, ib }
            }
            Scene::Gltf { gltf } => unimplemented!(),
        }
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
