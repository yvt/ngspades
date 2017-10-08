//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Vector3;
use cgmath::prelude::*;
use rand::{Rng, Rand};
use std::f32::consts::FRAC_1_PI;

/// Acoustic properties of an environment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct World<Q> {
    /// Absorption coefficients of the atmosphere (per unit distance).
    pub absorption: Q,

    /// Speed of sound, measured in unit distance per audio sample.
    ///
    /// For example, this value would be `7.78e-3` on a common setup where the
    /// sampling frequency is 44100 [Hz], the distance is measured in meters,
    /// and the standard Earth condition at 20 °C applies.
    pub speed_of_sound: f32,
}

/// Acoustic properties of a surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Material<Q> {
    /// Scattering coefficients (as defined by ISO 17497-1) for each frequency
    /// band.
    pub scatter: Q,

    /// Absorption coefficients (as defined by ISO 345) for each frquency band.
    pub absorption: Q,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaytraceHit<Q> {
    /// The position where the ray hit a surface.
    pub position: Vector3<f32>,

    /// The normal of the surface at the intersection position.
    pub normal: Vector3<f32>,

    /// The acoustic properties of the surface at the intersection position.
    pub material: Material<Q>,
}

/// Types that allows ray tracing on a certain environment.
pub trait Raytracer<Q> {
    /// Cast a ray from a given point.
    ///
    /// The ray direction `dir` must be a unit vector.
    ///
    /// Returns a `RaytraceHit` if the ray hits a surface.
    fn trace(&mut self, start: Vector3<f32>, dir: Vector3<f32>) -> Option<RaytraceHit<Q>>;

    /// Cast a ray with a finite length from a given point.
    ///
    /// Returns a `RaytraceHit` if the ray hits a surface.
    fn trace_finite(&mut self, start: Vector3<f32>, end: Vector3<f32>) -> Option<RaytraceHit<Q>> {
        let dir = (end - start).normalize();
        self.trace(start, dir).and_then(|hit| {
            let segment_len = (end - start).dot(dir);
            let hit_distance = (hit.position - start).dot(dir);
            if hit_distance <= segment_len {
                Some(hit)
            } else {
                None
            }
        })
    }

    /// Cast a ray from a given point to a random direction.
    ///
    /// Returns a `RaytraceHit` as well as the generation probability of the
    /// ray direction (which should be equal to 1/(4π)
    /// (= `std::f32::consts::FRAC_1_PI / 4.0`) if you are sampling uniformly)
    /// if the ray hits a surface.
    fn trace_sphere<R: Rng>(
        &mut self,
        start: Vector3<f32>,
        rng: &mut R,
    ) -> Option<(RaytraceHit<Q>, f32)> {
        self.trace(start, pick_sphere_point(rng)).map(|hit| {
            (hit, FRAC_1_PI / 4.0)
        })
    }

    /// Cast a ray from a given point to a random direction in a half sphere
    /// according to the Lambert Cosine distribution.
    ///
    /// Returns a `RaytraceHit` as well as the generation probability of the
    /// ray direction divided by cos θ (which should be equal to 1/π
    /// (= `std::f32::consts::FRAC_1_PI`) if you are really sampling
    /// according to the Lambert distribution) if the ray hits a surface.
    fn trace_lambert<R: Rng>(
        &mut self,
        start: Vector3<f32>,
        dir: Vector3<f32>,
        rng: &mut R,
    ) -> Option<(RaytraceHit<Q>, f32)> {
        let sph = pick_sphere_point(rng);
        let tangent = (sph - dir * sph.dot(dir) + dir * 1.0e-16).normalize();
        let x = <f32>::rand(rng);
        let l_dir = tangent * (1.0 - x * x).sqrt() + dir * x;
        self.trace(start, l_dir).map(|hit| (hit, FRAC_1_PI))
    }
}

fn pick_sphere_point<R: Rng>(rng: &mut R) -> Vector3<f32> {
    // Marsaglia, G. "Choosing a Point from the Surface of a Sphere." Ann. Math.
    // Stat. 43, 645-646, 1972.
    loop {
        let x1 = <f32>::rand(rng) * 2.0 - 1.0;
        let x2 = <f32>::rand(rng) * 2.0 - 1.0;
        let sq = x1 * x1 + x2 * x2;
        if sq < 1.0 {
            let t = 2.0 * (1.0 - sq).sqrt();
            break Vector3::new(t * x1, t * x2, 1.0 - 2.0 * sq);
        }
    }
}
