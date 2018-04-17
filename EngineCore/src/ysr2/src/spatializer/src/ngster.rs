//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! `Raytracer` implementation for a NgsTerrain terrain.
use std::borrow::Borrow;
use std::cmp::max;
use cgmath::Vector3;
use ngsterrain::{Terrain, raytrace, SolidVoxel};
use {Raytracer, RaytraceHit, Material};

/// Converts a given NgsTerrain material ID to a `Material`.
pub trait MaterialMap {
    type Quantity;

    /// Convert a given NgsTerrain material ID to `Material`.
    fn map(&self, material_id: u8) -> Material<Self::Quantity>;

    /// Return a flag indicating whether the return value of `map` is dependent
    /// on the given `material_id` or not.
    fn is_homogeneous(&self) -> bool {
        false
    }
}

/// A `MaterialMap` that maps all material IDs to a single material.
#[derive(Debug, Clone, Copy)]
pub struct ConstantMaterialMap<Q>(Material<Q>);

impl<Q> ConstantMaterialMap<Q> {
    /// Construct a `ConstantMaterialMap`.
    pub fn new(mat: Material<Q>) -> Self {
        ConstantMaterialMap(mat)
    }
}

impl<Q: Clone> MaterialMap for ConstantMaterialMap<Q> {
    type Quantity = Q;

    fn map(&self, _material_id: u8) -> Material<Q> {
        self.0.clone()
    }

    fn is_homogeneous(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct TerrainRaytracer<T, M> {
    terrain: T,
    material_map: M,
    scale: f32,
    scale_rcp: f32,
}

impl<T, M> TerrainRaytracer<T, M>
where
    T: Borrow<Terrain>,
    M: MaterialMap,
{
    /// Construct a new `TerrainRaytracer`.
    ///
    /// `scale` specifies the size of each voxel in the `Raytracer`'s coordinate
    /// space.
    pub fn new(terrain: T, material_map: M, scale: f32) -> Self {
        Self {
            terrain,
            material_map,
            scale,
            scale_rcp: scale.recip(),
        }
    }

    pub fn take_inner(self) -> (T, M) {
        (self.terrain, self.material_map)
    }
}

impl<T, M, Q> Raytracer<Q> for TerrainRaytracer<T, M>
where
    T: Borrow<Terrain>,
    M: MaterialMap<Quantity = Q>,
{
    fn trace(&mut self, start: Vector3<f32>, dir: Vector3<f32>) -> Option<RaytraceHit<Q>> {
        let len = {
            let terrain: &Terrain = self.terrain.borrow();
            let size = terrain.size();
            max(max(size.x, size.y), size.z) as f32 * self.scale * 2.0
        };
        self.trace_finite(start, start + dir * len)
    }

    fn trace_finite(&mut self, start: Vector3<f32>, end: Vector3<f32>) -> Option<RaytraceHit<Q>> {
        use self::raytrace::RaytraceResult::*;
        let terrain: &Terrain = self.terrain.borrow();

        match raytrace::raytrace(terrain, start * self.scale_rcp, end * self.scale_rcp) {
            Hit(hit) => {
                let material_id = if self.material_map.is_homogeneous() {
                    0
                } else {
                    let voxel = terrain.get_voxel(hit.voxel).unwrap();
                    match voxel {
                        SolidVoxel::Colored(colored) => *colored.material(),
                        SolidVoxel::Uncolored => 0,
                    }
                };

                let material = self.material_map.map(material_id);
                Some(RaytraceHit {
                    position: hit.position * self.scale,
                    normal: hit.normal.as_vector3(),
                    material,
                })
            }
            NoHit | Inside(_) => None,
        }
    }
}
