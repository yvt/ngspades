//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use rand::{Rng, Rand};
use cgmath::Vector3;
use cgmath::prelude::*;
use {Raytracer, World, BaseFdQuant, RaytraceHit};
use flattener::Flattener;

/// Perform an acoustic path-tracing with the diffuse rain technique (PT-DR) and
/// record zero or more impulses to the given `Flattener`.
///
/// The detector is assumed to be infinitely small (similar to the visual
/// rendering's pinhole camera model). As a result, it is impossible to detect
/// pure-specular paths, which are important for early reflections. Combine it
/// with other techniques (e.g., image-source method) to compensate this
/// restriction.
///
/// A single ray is generated for every call to this function.
///
/// Returns the length of the generated path (excluding the starting point).
pub fn raytrace_ptdr<Q, T, R, F>(
    tracer: &mut T,
    flattener: &mut F,
    world: &World<Q>,
    listener_pos: Vector3<f32>,
    source_pos: Vector3<f32>,
    amplitude: Q::Scalar,
    rng: &mut R,
    max_reflections: usize,
) -> usize
where
    Q: BaseFdQuant,
    T: Raytracer<Q>,
    R: Rng,
    F: Flattener<Quantity = Q>,
{
    use std::f32::consts::FRAC_1_PI;
    use cgmath::num_traits::NumCast;
    use cgmath::num_traits::ToPrimitive;

    let sos_recip = world.speed_of_sound.recip();

    let mut start = source_pos;
    let mut traveled_distance = 0.0;
    let mut energy = Q::one() * amplitude;

    let mut result: RaytraceHit<_> = if let Some((hit, pd)) = tracer.trace_sphere(start, rng) {
        let weight = 0.25 * FRAC_1_PI / pd;
        energy *= <Q::Scalar as NumCast>::from(weight).unwrap();
        hit
    } else {
        return 0;
    };

    for i in 0..max_reflections {
        // Absorption
        energy *= Q::one() - result.material.absorption;

        traveled_distance += (result.position - start).magnitude();

        // Diffuse rain
        if tracer.trace_finite(result.position, listener_pos).is_none() {
            let final_distance = traveled_distance + (result.position - listener_pos).magnitude();
            let mut final_energy = energy * result.material.scatter;
            let fd_qs = <Q::Scalar as NumCast>::from(-final_distance).unwrap();
            final_energy *= (world.absorption * fd_qs).exp();

            flattener.record_imp_dir(
                final_distance * sos_recip,
                final_energy,
                result.position - listener_pos,
            );
        }

        if i < max_reflections - 1 {
            // Choose the next path
            let scatter_prob_qs = result.material.scatter.average();
            let scatter_prob = scatter_prob_qs.to_f32().unwrap();
            let dice = <f32>::rand(rng);

            if dice < scatter_prob {
                energy *= result.material.scatter / scatter_prob_qs;

                start = result.position;
                result = if let Some((hit, pd)) = tracer.trace_lambert(start, result.normal, rng) {
                    let weight = FRAC_1_PI / pd;
                    energy *= <Q::Scalar as NumCast>::from(weight).unwrap();
                    hit
                } else {
                    return i + 1;
                };
            } else {
                energy *= (Q::one() - result.material.scatter) /
                    (Q::Scalar::one() - scatter_prob_qs);

                let vi = (result.position - start).normalize();
                let vr = vi - result.normal * (result.normal.dot(vi) * 2.0);

                start = result.position;
                result = if let Some(hit) = tracer.trace(start, vr) {
                    hit
                } else {
                    return i + 1;
                };
            }
        }
    }

    max_reflections
}
