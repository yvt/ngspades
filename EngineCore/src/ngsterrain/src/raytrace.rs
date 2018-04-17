//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! A naïve ray tracer for `Terrain`.
use cgmath::{Vector3, vec3};
use cgmath::prelude::*;
use std::cmp;
use std::borrow::Borrow;

use {Terrain, Row, CubeFace};
use geom;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaytraceHit {
    pub voxel: Vector3<usize>,
    pub normal: CubeFace,
    pub position: Vector3<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RaytraceResult {
    /// The starting point was in a solid voxel.
    Inside(Vector3<usize>),

    /// The ray hit a surface.
    Hit(RaytraceHit),

    /// The ray didn't hit a surface.
    NoHit,
}

/// Perform a ray tracing on a given `Terrain`.
///
/// All given coordinates must be finite and within the range
/// `[-0x7fffffff, 0x7fffffff]`.
pub fn raytrace(terrain: &Terrain, start: Vector3<f32>, to: Vector3<f32>) -> RaytraceResult {
    let size = terrain.size();
    let size_f = size.cast::<f32>();

    // Is the starting point inside a solid voxel?
    let start_inside = start.x >= 0.0 && start.y >= 0.0 && start.z >= 0.0 &&
        start.x < size_f.x && start.y < size_f.y && start.z < size_f.z;

    if start_inside {
        let start_i = Vector3::new(
            truncate_toward(start.x, to.x >= start.x),
            truncate_toward(start.y, to.y >= start.y),
            truncate_toward(start.z, to.z >= start.z),
        );
        if terrain.get_voxel(start_i).is_some() {
            return RaytraceResult::Inside(start_i);
        }
    }

    if start == to {
        return RaytraceResult::NoHit;
    }

    if start.truncate() == to.truncate() {
        // Parallel with the Z axis
        let z_min = start.z.min(to.z).max(0.0);
        let z_max = start.z.max(to.z).min(size_f.z - 1.0);
        if z_min >= size_f.z || z_max < 0.0 {
            return RaytraceResult::NoHit;
        }
        let row_coord = start.truncate().cast();
        let row = terrain.get_row(row_coord).unwrap();
        let z_hit = if to.z > start.z {
            cast_row(&row, z_min as usize, z_max as usize, true)
        } else {
            cast_row(&row, z_max as usize, z_min as usize, false)
        };
        if let Some(z_hit) = z_hit {
            let z_hit_f = if to.z > start.z {
                z_hit as f32
            } else {
                (z_hit + 1) as f32
            };
            let mut hit_pos = start.lerp(to, (z_hit_f - start.z) / (to.z - start.z));
            hit_pos.z = z_hit_f;
            return RaytraceResult::Hit(RaytraceHit {
                voxel: Vector3::new(row_coord.x, row_coord.y, z_hit),
                normal: if to.z > start.z {
                    CubeFace::NegativeZ
                } else {
                    CubeFace::PositiveZ
                },
                position: hit_pos,
            });
        } else {
            return RaytraceResult::NoHit;
        }
    }

    let (entering_face, start) = if start_inside {
        (None, start)
    } else {
        // Find the point where the ray enters the AABB of the terrain
        if let Some(x) = geom::clip_ray_start_by_aabb(start, to, Vector3::zero(), size_f) {
            x
        } else {
            return RaytraceResult::NoHit;
        }
    };

    let x_pos = to.x >= start.x;
    let y_pos = to.y >= start.y;
    let z_pos = to.z >= start.z;

    let dir = to - start;
    let y_major = dir.y.abs() > dir.x.abs();

    let entering_face = if let Some(face) = entering_face {
        match face {
            CubeFace::PositiveX | CubeFace::NegativeX => HitFace::X,
            CubeFace::PositiveY | CubeFace::NegativeY => HitFace::Y,
            CubeFace::PositiveZ | CubeFace::NegativeZ => HitFace::Z,
        }
    } else {
        if y_major { HitFace::Y } else { HitFace::X }
    };

    if y_major {
        if z_pos {
            raytrace_inner(
                terrain,
                start,
                to,
                vec3(x_pos, y_pos, true),
                true,
                entering_face,
            )
        } else {
            raytrace_inner(
                terrain,
                start,
                to,
                vec3(x_pos, y_pos, false),
                true,
                entering_face,
            )
        }
    } else {
        if z_pos {
            raytrace_inner(
                terrain,
                start,
                to,
                vec3(x_pos, y_pos, true),
                false,
                entering_face,
            )
        } else {
            raytrace_inner(
                terrain,
                start,
                to,
                vec3(x_pos, y_pos, false),
                false,
                entering_face,
            )
        }
    }
}

enum HitFace {
    X,
    Y,
    Z,
}

#[inline(always)]
fn raytrace_inner(
    terrain: &Terrain,
    start: Vector3<f32>,
    to: Vector3<f32>,
    positive: Vector3<bool>,
    y_major: bool,
    entering_face: HitFace,
) -> RaytraceResult {
    match raytrace_inner_2(terrain, start, to, positive, y_major, entering_face) {
        Some((voxel, HitFace::X)) => {
            let hit_x_f = if positive.x { voxel.x } else { voxel.x + 1 } as f32;
            let t = (hit_x_f - start.x) / (to.x - start.x);
            if t <= 1.0 {
                let mut position = start.lerp(to, t);
                position.x = hit_x_f;
                RaytraceResult::Hit(RaytraceHit {
                    voxel,
                    normal: if positive.x {
                        CubeFace::NegativeX
                    } else {
                        CubeFace::PositiveX
                    },
                    position,
                })
            } else {
                RaytraceResult::NoHit
            }
        }
        Some((voxel, HitFace::Y)) => {
            let hit_y_f = if positive.y { voxel.y } else { voxel.y + 1 } as f32;
            let t = (hit_y_f - start.y) / (to.y - start.y);
            if t <= 1.0 {
                let mut position = start.lerp(to, t);
                position.y = hit_y_f;
                RaytraceResult::Hit(RaytraceHit {
                    voxel,
                    normal: if positive.y {
                        CubeFace::NegativeY
                    } else {
                        CubeFace::PositiveY
                    },
                    position,
                })
            } else {
                RaytraceResult::NoHit
            }
        }
        Some((voxel, HitFace::Z)) => {
            let hit_z_f = if positive.z { voxel.z } else { voxel.z + 1 } as f32;
            let t = (hit_z_f - start.z) / (to.z - start.z);
            if t <= 1.0 {
                let mut position = start.lerp(to, t);
                position.z = hit_z_f;
                RaytraceResult::Hit(RaytraceHit {
                    voxel,
                    normal: if positive.z {
                        CubeFace::NegativeZ
                    } else {
                        CubeFace::PositiveZ
                    },
                    position,
                })
            } else {
                RaytraceResult::NoHit
            }
        }
        None => RaytraceResult::NoHit,
    }
}

#[inline(always)]
fn raytrace_inner_2(
    terrain: &Terrain,
    start: Vector3<f32>,
    to: Vector3<f32>,
    positive: Vector3<bool>,
    y_major: bool,
    entering_face: HitFace,
) -> Option<(Vector3<usize>, HitFace)> {
    let size = terrain.size();

    let mut cur_z = start.z;
    let mut cur_pos = Vector3::new(
        truncate_toward(start.x, positive.x),
        truncate_toward(start.y, positive.y),
        truncate_toward(start.z, positive.z),
    );

    debug_assert!(cur_pos.x < size.x);
    debug_assert!(cur_pos.y < size.y);
    debug_assert!(cur_pos.z < size.z);

    let dx = if positive.x { 1 } else { !0usize };
    let dy = if positive.y { 1 } else { !0usize };
    let dz = if positive.z { 1 } else { !0usize };
    let d = Vector3::new(dx, dy, dz);

    let mut dir = to - start;

    // Indices of major/minor axises
    let major_i = if y_major { 1 } else { 0 };
    let minor_i = 1 - major_i;

    let mut dist_major_abs = dir[major_i].abs();

    // Normalize the major axis (so its absolute value becomes `1`)
    // The minor axis should be within `[-1, 1]`
    dir[minor_i] /= dir[major_i].abs();
    dir[2] /= dir[major_i].abs();
    dir[major_i] = dir[major_i].signum();

    let dir_minor_abs = dir[minor_i].abs();
    debug_assert!(dir_minor_abs <= 1.0, "dir = {:?}", dir);

    let recip_dir_minor_abs = dir_minor_abs.recip();

    let mut residue_minor =
        ((cur_pos[minor_i] + if positive[minor_i] { 1 } else { 0 }) as f32 - start[minor_i]).abs();

    {
        let residue_major = ((cur_pos[major_i] + if positive[major_i] { 1 } else { 0 }) as f32 -
                                 start[major_i])
            .abs();

        // Check the first slice
        if residue_major != 0.0 {
            let t = residue_major;

            if residue_minor < dir_minor_abs * t {
                // Also enters a neighbor row in the minor axis' direction
                let et = residue_minor * recip_dir_minor_abs;
                let new_z = truncate_toward((cur_z + et * dir.z).max(0.0), positive.z);

                let row = terrain.get_row(cur_pos.truncate()).unwrap();
                if let Some(z) = cast_row(&row, cur_pos.z, new_z, positive.z) {
                    return Some((
                        cur_pos.truncate().extend(z),
                        if z == cur_pos.z {
                            entering_face
                        } else {
                            HitFace::Z
                        },
                    ));
                }
                cur_pos.z = new_z;

                if et >= dist_major_abs {
                    return None;
                }

                // Enter the neighbor row...
                residue_minor -= dir_minor_abs * t;
                residue_minor += 1.0;
                cur_pos[minor_i] = cur_pos[minor_i].wrapping_add(d[minor_i]);
                if cur_pos[minor_i] >= size[minor_i] {
                    return None;
                }
                cur_z += t * dir.z;
                let new_z = truncate_toward(cur_z.max(0.0), positive.z);

                let row = terrain.get_row(cur_pos.truncate()).unwrap();
                if let Some(z) = cast_row(&row, cur_pos.z, new_z, positive.z) {
                    return Some((
                        cur_pos.truncate().extend(z),
                        if z == cur_pos.z {
                            if y_major { HitFace::X } else { HitFace::Y }
                        } else {
                            HitFace::Z
                        },
                    ));
                }
                cur_pos.z = new_z;
            } else {
                residue_minor -= dir_minor_abs * t;
                cur_z += t * dir.z;
                let new_z = truncate_toward(cur_z.max(0.0), positive.z);

                let row = terrain.get_row(cur_pos.truncate()).unwrap();
                if let Some(z) = cast_row(&row, cur_pos.z, new_z, positive.z) {
                    return Some((
                        cur_pos.truncate().extend(z),
                        if z == cur_pos.z {
                            entering_face
                        } else {
                            HitFace::Z
                        },
                    ));
                }
                cur_pos.z = new_z;
            }

            dist_major_abs -= t;

            cur_pos[major_i] = cur_pos[major_i].wrapping_add(d[major_i]);
            if cur_pos[major_i] >= size[major_i] {
                return None;
            }
            if if positive.z {
                cur_z >= size.z as f32
            } else {
                cur_z < 0.0
            }
            {
                return None;
            }
        }
    }

    // Check each slice
    while dist_major_abs > 0.0 {
        if residue_minor < dir_minor_abs {
            // Also enters a neighbor row in the minor axis' direction
            let et = residue_minor * recip_dir_minor_abs;
            let new_z = truncate_toward((cur_z + et * dir.z).max(0.0), positive.z);

            let row = terrain.get_row(cur_pos.truncate()).unwrap();
            if let Some(z) = cast_row(&row, cur_pos.z, new_z, positive.z) {
                return Some((
                    cur_pos.truncate().extend(z),
                    if z == cur_pos.z {
                        if y_major { HitFace::Y } else { HitFace::X }
                    } else {
                        HitFace::Z
                    },
                ));
            }
            cur_pos.z = new_z;

            if et >= dist_major_abs {
                return None;
            }

            // Enter the neighbor row...
            residue_minor = residue_minor - dir_minor_abs + 1.0;
            cur_pos[minor_i] = cur_pos[minor_i].wrapping_add(d[minor_i]);
            if cur_pos[minor_i] >= size[minor_i] {
                return None;
            }
            cur_z += dir.z;
            let new_z = truncate_toward(cur_z.max(0.0), positive.z);

            let row = terrain.get_row(cur_pos.truncate()).unwrap();
            if let Some(z) = cast_row(&row, cur_pos.z, new_z, positive.z) {
                return Some((
                    cur_pos.truncate().extend(z),
                    if z == cur_pos.z {
                        if y_major { HitFace::X } else { HitFace::Y }
                    } else {
                        HitFace::Z
                    },
                ));
            }
            cur_pos.z = new_z;
        } else {
            residue_minor -= dir_minor_abs;
            cur_z += dir.z;
            let new_z = truncate_toward(cur_z.max(0.0), positive.z);

            let row = terrain.get_row(cur_pos.truncate()).unwrap();
            if let Some(z) = cast_row(&row, cur_pos.z, new_z, positive.z) {
                return Some((
                    cur_pos.truncate().extend(z),
                    if z == cur_pos.z {
                        if y_major { HitFace::Y } else { HitFace::X }
                    } else {
                        HitFace::Z
                    },
                ));
            }
            cur_pos.z = new_z;
        }

        dist_major_abs -= 1.0;
        cur_pos[major_i] = cur_pos[major_i].wrapping_add(d[major_i]);
        if cur_pos[major_i] >= size[major_i] {
            return None;
        }

        if if positive.z {
            cur_z >= size.z as f32
        } else {
            cur_z < 0.0
        }
        {
            return None;
        }
    }

    None
}

fn truncate_toward(x: f32, pos: bool) -> usize {
    if pos {
        x.floor() as usize
    } else {
        (x.ceil() as usize).saturating_sub(1)
    }
}

#[inline(always)]
fn cast_row<T: Borrow<[u8]>>(
    row: &Row<&T>,
    start_z: usize,
    end_z: usize,
    positive: bool,
) -> Option<usize> {
    if positive {
        debug_assert!(end_z >= start_z);
        for range in row.chunk_z_ranges() {
            if range.start > end_z {
                return None;
            } else if range.end > start_z {
                return Some(cmp::max(start_z, range.start));
            }
        }
        None
    } else {
        debug_assert!(end_z <= start_z);
        let mut ret = None;
        for range in row.chunk_z_ranges() {
            if range.start > start_z {
                break;
            } else if range.end > end_z {
                ret = Some(range.end);
            }
        }
        ret.map(|x| cmp::min(x - 1, start_z))
    }
}
