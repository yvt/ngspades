//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use alt_fp::FloatOrdSet;
use array::Array3;
use cgmath::{
    conv::{array3, array4x4},
    prelude::*,
    vec2, Matrix4, Point3,
};
use glium::{
    backend::Facade,
    index::{IndexBufferAny, PrimitiveType},
    program, uniform,
    vertex::VertexBufferAny,
    IndexBuffer, Program, Surface, VertexBuffer,
};
use gltf;
use ngsterrain;
use pod::Pod;
use std::{cmp::min, collections::HashMap, ffi::OsStr, fs::File, io::Read, path::Path};
use xz_decom::decompress;

use crate::lib::{profmempool, terrainload, vxl2mesh};
use stygian::{gen, mempool};

#[derive(Debug)]
pub enum Scene {
    NgsTerrain {
        ngs_terrain: ngsterrain::Terrain,
    },
    Gltf {
        gltf: gltf::Gltf,
        scene_index: usize,
    },
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

            let scene = gltf.scenes().nth(0).expect("no scene was found");
            let scene_index = scene.index();
            println!("Scene name: {:?}", scene.name());

            Scene::Gltf { gltf, scene_index }
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
            Scene::Gltf { gltf, scene_index } => {
                let scene = gltf
                    .scenes()
                    .filter(|s| s.index() == *scene_index)
                    .nth(0)
                    .unwrap();

                let [ms_min, ms_max] = gltf_get_aabb(gltf, &scene);

                const SMALLEST_HOLE: f32 = 0.25;
                const SMALLEST_OCCLUDER: f32 = 1.0;

                let size = ms_max - ms_min;
                let mut terrain_size = (size / (SMALLEST_HOLE * 0.5)).cast::<u32>().unwrap();
                terrain_size.x = terrain_size.x.next_power_of_two();
                terrain_size.y = terrain_size.y.next_power_of_two();
                terrain_size.z = terrain_size.z.next_power_of_two();
                println!("Initial domain size: {:?}", terrain_size);

                let mut downsample_bits = 0;
                let cell_size = [
                    size.x / terrain_size.x as f32,
                    size.y / terrain_size.y as f32,
                ]
                .fmax();
                loop {
                    downsample_bits += 1;
                    if cell_size * ((3 << downsample_bits) + 2) as f32 > SMALLEST_OCCLUDER {
                        downsample_bits -= 1;
                        break;
                    }
                }
                println!("Smallest hole size: {:?}", cell_size * 2.0);
                println!(
                    "Downsampled domain size: {:?}",
                    [
                        terrain_size.x >> downsample_bits,
                        terrain_size.y >> downsample_bits,
                        terrain_size.z,
                    ]
                );
                println!(
                    "Smallest occluder size: {:?}",
                    cell_size * ((3 << downsample_bits) + 2) as f32
                );

                let matrix = Matrix4::from_nonuniform_scale(
                    terrain_size.x as f32 / size.x,
                    terrain_size.y as f32 / size.y,
                    terrain_size.z as f32 / size.z,
                ) * Matrix4::from_translation(Point3::new(0.0, 0.0, 0.0) - ms_min);

                let tile_size_bits = min(terrain_size.x, terrain_size.y).trailing_zeros();
                let domain = gen::InitialDomain {
                    tile_size_bits,
                    tile_count: vec2(
                        terrain_size.x >> tile_size_bits,
                        terrain_size.y >> tile_size_bits,
                    ),
                    depth: terrain_size.z,
                };
                dbg!(&domain);

                let pool = profmempool::ProfMemPool::new(mempool::SysMemPool);

                println!("binning...");
                let mut binned_geometry = gen::BinnedGeometry::new(&pool, &domain);
                {
                    let mut binner = gen::PolygonBinner::new(1024, &domain, &mut binned_geometry);

                    gltf_enum_triangles(gltf, &scene, |vertices| {
                        binner.insert(vertices.map(|v| matrix.transform_point(v)));
                    });

                    binner.flush();
                }

                println!("from_geometry...");
                let mut voxels = gen::VoxelBitmap::from_geometry(&pool, &domain, &binned_geometry);

                println!("flood_fill_in_place...");
                voxels.flood_fill_in_place(
                    &domain,
                    &[Point3::new(
                        terrain_size.x / 2,
                        terrain_size.y / 2,
                        terrain_size.z - 1,
                    )],
                    gen::VoxelType::Empty,
                    gen::VoxelType::View,
                );

                println!("erode_view...");
                let voxels2 = voxels.erode_view(&pool, &domain);

                println!("to_terrain...");
                let downsample = (1 << downsample_bits) as f32;
                (
                    voxels2.to_terrain(&domain, downsample_bits).unwrap(),
                    matrix.invert().unwrap()
                        * Matrix4::from_nonuniform_scale(downsample, downsample, 1.0),
                )
            }
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
            Scene::Gltf { gltf, scene_index } => {
                let scene = gltf
                    .scenes()
                    .filter(|s| s.index() == *scene_index)
                    .nth(0)
                    .unwrap();

                let [min, max] = gltf_get_aabb(gltf, &scene);
                println!("Bounding box: {:?}–{:?}", min, max);

                Point3::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5, max.z)
            }
        }
    }
}

fn gltf_get_aabb(gltf: &gltf::Gltf, scene: &gltf::scene::Scene) -> [Point3<f32>; 2] {
    use std::f32::{INFINITY, NEG_INFINITY};
    let mut min = [INFINITY; 3];
    let mut max = [NEG_INFINITY; 3];

    gltf_enum_triangles(gltf, scene, |vertices| {
        for &vert in &vertices {
            for (i, &x) in array3(vert).iter().enumerate() {
                min[i] = [min[i], x].fmin();
                max[i] = [max[i], x].fmax();
            }
        }
    });

    [min.into(), max.into()]
}

fn gltf_enum_triangles(
    gltf: &gltf::Gltf,
    scene: &gltf::scene::Scene,
    mut tri_callback: impl FnMut([Point3<f32>; 3]),
) {
    let blob = gltf.blob.as_ref().unwrap();
    for node in scene.nodes() {
        if let Some(mesh) = node.mesh() {
            let m = Matrix4::from(node.transform().matrix());
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

                    tri_callback(positions.map(|p| m.transform_point(p)));
                }
            }
        }
    }
}

fn vertex_fetch<T: Pod>(view: &gltf::buffer::View, blob: &[u8], i: usize) -> T {
    let size = std::mem::size_of::<T>();
    let stride = view.stride().unwrap_or(size);
    let start = view.offset() + stride * i;
    T::from_bytes(&blob[start..start + size]).unwrap()
}

#[derive(Debug)]
pub struct SceneRenderer {
    program: [Program; 2],
}

impl SceneRenderer {
    pub fn new(facade: &impl Facade) -> Self {
        let program1 = program!(facade,
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
            }
        )
        .unwrap();

        // No vertex color
        let program2 = program!(facade,
            100 => {
                vertex: r"
                    #version 100

                    uniform highp mat4 u_matrix;
                    attribute highp vec3 pos;
                    attribute highp vec3 norm;
                    varying lowp vec4 v_color;

                    void main() {
                        v_color = vec4(0.9, 0.9, 0.9, 1.0);
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
            }
        )
        .unwrap();
        SceneRenderer {
            program: [program1, program2],
        }
    }
}

#[derive(Debug)]
pub struct SceneInstance {
    meshes: Vec<Mesh>,
    mesh_instances: Vec<MeshInstance>,
}

#[derive(Debug)]
struct Mesh {
    vb: Vec<VertexBufferAny>,
    ib: IndexBufferAny,
    program: usize,
}

#[derive(Debug)]
struct MeshInstance {
    mesh_index: usize,
    transform: Matrix4<f32>,
}

impl SceneRenderer {
    pub fn prepare_scene(&self, facade: &impl Facade, scene: &Scene) -> SceneInstance {
        match scene {
            Scene::NgsTerrain { ngs_terrain } => {
                // Convert the terrain to a mesh
                println!("Converting the terrain into a mesh");
                let (verts, indices) = vxl2mesh::terrain_to_mesh(ngs_terrain);
                let vb = VertexBuffer::new(facade, &verts).unwrap();
                let ib = IndexBuffer::new(facade, PrimitiveType::TrianglesList, &indices).unwrap();

                let mesh = Mesh {
                    vb: vec![vb.into()],
                    ib: ib.into(),
                    program: 0,
                };

                SceneInstance {
                    meshes: vec![mesh],
                    mesh_instances: vec![MeshInstance {
                        mesh_index: 0,
                        transform: Matrix4::identity(),
                    }],
                }
            }
            Scene::Gltf { gltf, scene_index } => {
                let mut meshes = Vec::new();
                let mut index_map = HashMap::new();

                let blob = gltf.blob.as_ref().unwrap();

                for gltf_mesh in gltf.meshes() {
                    let mut mesh_indices = Vec::new();
                    for prim in gltf_mesh.primitives() {
                        let positions = prim.get(&gltf::Semantic::Positions).unwrap();
                        let normals = prim.get(&gltf::Semantic::Normals).expect("normals missing");
                        let indices = prim.indices().unwrap();

                        // I couldn't find a way to slice portions of a single buffer
                        // for all vertex attributes, so we simply create
                        // `VertexBuffer` for every vertex attribute
                        let positions = vb_from_gltf_accessor(facade, blob, &positions, "pos");
                        let normals = vb_from_gltf_accessor(facade, blob, &normals, "norm");

                        mesh_indices.push(meshes.len());
                        meshes.push(Mesh {
                            vb: vec![positions, normals],
                            ib: ib_from_gltf_accessor(facade, blob, &indices),
                            program: 1,
                        });
                    }
                    index_map.insert(gltf_mesh.index(), mesh_indices);
                }

                let mut mesh_instances = Vec::new();

                let scene = gltf
                    .scenes()
                    .filter(|s| s.index() == *scene_index)
                    .nth(0)
                    .unwrap();

                for gltf_node in scene.nodes() {
                    if let Some(mesh) = gltf_node.mesh() {
                        let m = Matrix4::from(gltf_node.transform().matrix());

                        for &mesh_index in index_map.get(&mesh.index()).unwrap() {
                            mesh_instances.push(MeshInstance {
                                mesh_index: mesh_index,
                                transform: m,
                            });
                        }
                    }
                }

                SceneInstance {
                    meshes,
                    mesh_instances,
                }
            }
        }
    }

    pub fn draw_scene(
        &self,
        instance: &SceneInstance,
        target: &mut impl Surface,
        pvm_matrix: Matrix4<f32>,
    ) {
        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::DepthTest::IfMore,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };

        for mesh_instance in instance.mesh_instances.iter() {
            let uniforms = uniform! {
                u_matrix: array4x4(pvm_matrix * mesh_instance.transform),
            };

            let mesh = &instance.meshes[mesh_instance.mesh_index];

            match &mesh.vb[..] {
                [vb1] => {
                    target
                        .draw(
                            vb1,
                            &mesh.ib,
                            &self.program[mesh.program],
                            &uniforms,
                            &params,
                        )
                        .unwrap();
                }
                [vb1, vb2] => {
                    target
                        .draw(
                            (vb1, vb2),
                            &mesh.ib,
                            &self.program[mesh.program],
                            &uniforms,
                            &params,
                        )
                        .unwrap();
                }
                _ => unreachable!(),
            }
        }
    }
}

fn vb_from_gltf_accessor(
    facade: &impl Facade,
    blob: &[u8],
    accessor: &gltf::accessor::Accessor,
    name: &'static str,
) -> VertexBufferAny {
    use glium::vertex::AttributeType;
    use gltf::accessor::{DataType, Dimensions};

    let uninterleaved = uninterleave_gltf_accessor(blob, accessor);

    let attrs = vec![(
        name.into(),
        0,
        match (accessor.data_type(), accessor.dimensions()) {
            (DataType::F32, Dimensions::Vec3) => AttributeType::F32F32F32,
            x => panic!("unsupported vertex type: {:?}", x),
        },
        false,
    )];

    // Work-around for <https://github.com/glium/glium/issues/1461>
    // `new_raw` guesses stride from the type parameter, not `element_size`.
    unsafe {
        match accessor.size() {
            12 => VertexBuffer::new_raw(
                facade,
                Pod::map_slice::<[u32; 3]>(&uninterleaved).unwrap(),
                attrs.into(),
                accessor.size(),
            )
            .unwrap()
            .into(),
            _ => unreachable!(),
        }
    }
}

fn ib_from_gltf_accessor(
    facade: &impl Facade,
    blob: &[u8],
    accessor: &gltf::accessor::Accessor,
) -> IndexBufferAny {
    let uninterleaved = uninterleave_gltf_accessor(blob, accessor);
    assert_eq!(accessor.data_type(), gltf::accessor::DataType::U16);
    IndexBuffer::new(
        facade,
        PrimitiveType::TrianglesList,
        Pod::map_slice::<u16>(&uninterleaved).unwrap(),
    )
    .unwrap()
    .into()
}

fn uninterleave_gltf_accessor(blob: &[u8], accessor: &gltf::accessor::Accessor) -> Vec<u8> {
    let size = accessor.size();
    let count = accessor.count();
    let offset = accessor.view().offset();
    let stride = accessor.view().stride().unwrap_or(size);

    let mut dense = vec![0u8; size * count];
    for i in 0..count {
        dense[i * size..][..size].copy_from_slice(&blob[offset + i * stride..][..size]);
    }

    dense
}
