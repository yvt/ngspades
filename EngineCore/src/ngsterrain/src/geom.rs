//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Vector3;
use cgmath::prelude::*;
use std::cmp::Ordering;

use CubeFace;

pub fn clip_ray_start_by_aabb(
    start: Vector3<f32>,
    end: Vector3<f32>,
    box_min: Vector3<f32>,
    box_max: Vector3<f32>,
) -> Option<(Option<CubeFace>, Vector3<f32>)> {
    #[inline(always)]
    fn clip_one(
        last_side: Option<CubeFace>,
        start: Vector3<f32>,
        end: Vector3<f32>,
        box_min: Vector3<f32>,
        box_max: Vector3<f32>,
        i: usize,
    ) -> Option<(Option<CubeFace>, Vector3<f32>)> {
        match end[i].partial_cmp(&start[i]) {
            Some(Ordering::Greater) => {
                if start[i] >= box_max[i] {
                    None
                } else if start[i] < box_min[i] {
                    if end[i] < box_min[i] {
                        None
                    } else {
                        let per = (box_min[i] - start[i]) / (end[i] - start[i]);
                        let side = match i {
                            0 => CubeFace::NegativeX,
                            1 => CubeFace::NegativeY,
                            2 => CubeFace::NegativeZ,
                            _ => unreachable!(),
                        };
                        let mut new = start.lerp(end, per);
                        new[i] = box_min[i];
                        Some((Some(side), new))
                    }
                } else {
                    Some((last_side, start))
                }
            }
            Some(Ordering::Less) => {
                if start[i] <= box_min[i] {
                    None
                } else if start[i] > box_max[i] {
                    if end[i] > box_max[i] {
                        None
                    } else {
                        let per = (box_max[i] - start[i]) / (end[i] - start[i]);
                        let side = match i {
                            0 => CubeFace::PositiveX,
                            1 => CubeFace::PositiveY,
                            2 => CubeFace::PositiveZ,
                            _ => unreachable!(),
                        };
                        let mut new = start.lerp(end, per);
                        new[i] = box_max[i];
                        Some((Some(side), new))
                    }
                } else {
                    Some((last_side, start))
                }
            }
            _ => {
                if start[i] >= box_min[i] || start[i] < box_max[i] {
                    Some((last_side, start))
                } else {
                    None
                }
            }
        }
    }

    #[inline(always)]
    fn check_one(
        start: Vector3<f32>,
        end: Vector3<f32>,
        box_min: Vector3<f32>,
        box_max: Vector3<f32>,
        i: usize,
    ) -> bool {
        if end[i] >= start[i] {
            start[i] >= box_min[i] && start[i] < box_max[i]
        } else {
            start[i] > box_min[i] && start[i] <= box_max[i]
        }
    }

    clip_one(None, start, end, box_min, box_max, 0)
        .and_then(|(side, start)| {
            clip_one(side, start, end, box_min, box_max, 1)
        })
        .and_then(|(side, start)| {
            clip_one(side, start, end, box_min, box_max, 2)
        })
        .and_then(
            |(side, start)| if check_one(start, end, box_min, box_max, 0) &&
                check_one(start, end, box_min, box_max, 1) &&
                check_one(start, end, box_min, box_max, 2)
            {
                Some((side, start))
            } else {
                None
            },
        )
}
