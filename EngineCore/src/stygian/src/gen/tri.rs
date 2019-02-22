//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! The conservative rasterizer for triangles.
use alt_fp::{fma, FloatOrdSet};
use cgmath::{prelude::*, Point2, Point3, Vector2};
use std::{
    cmp::{max, min},
    f32::{INFINITY, NEG_INFINITY},
    mem::swap,
    ops::Range,
};

/// Perform conservative rasterization for a given triangle.
///
/// For each output pixel, a range of Z values is calculated.
/// Each produced scanline is passed to `scanline_handler`. The first parameter
/// represents the coordinates `p` of the left-most produced point of the
/// scanline.
/// The `i`-th element of the second parameter represents the range of Z values
/// at the cell `(p.x + i, p.y)`
///
/// The viewport region is `x ∈ [0, size.x), y ∈ [0, size.y)`.
/// `z_buffer` is an intermediate buffer and thus needn't be initialized.
/// However, it must have at least `size.x` elements.
pub fn tricrast(
    vertices: [Point3<f32>; 3],
    size: Vector2<u32>,
    z_buffer: &mut [Range<f32>],
    mut scanline_handler: impl FnMut(Point2<u32>, &mut [Range<f32>]),
) {
    let [mut v0, mut v1, mut v2] = vertices;

    // Sort by Y coordinates
    if v1.y < v0.y {
        swap(&mut v0, &mut v1);
    }
    if v2.y < v1.y {
        swap(&mut v1, &mut v2);
    }
    if v1.y < v0.y {
        swap(&mut v0, &mut v1);
    }

    assert!(z_buffer.len() >= size.x as usize);
    let z_buffer = &mut z_buffer[0..size.x as usize];
    if size.is_zero() {
        return;
    }

    // The Y coordinates convert to integers
    let v0yi = v0.y.floor() as i32;
    let v1yi = v1.y.floor() as i32;
    let v2yi = v2.y.floor() as i32;

    // The current scanline
    let mut yi = max(0, v0yi);

    if yi >= size.y as i32 {
        return;
    }

    let dx01 = (v1.x - v0.x) / (v1.y - v0.y);
    let dz01 = (v1.z - v0.z) / (v1.y - v0.y);

    let dx12 = (v2.x - v1.x) / (v2.y - v1.y);
    let dz12 = (v2.z - v1.z) / (v2.y - v1.y);

    let dx02 = (v2.x - v0.x) / (v2.y - v0.y);
    let dz02 = (v2.z - v0.z) / (v2.y - v0.y);

    // 0 → 1 → 2
    let (mut side_x1, mut side_z1);
    // 0 → 2
    let (mut side_x2, mut side_z2);

    'top_half: loop {
        if yi == v0yi {
            if yi == v2yi {
                debug_assert_eq!(v2yi, yi);

                if let Some((origin, range)) = init_scanline(yi, z_buffer, &[v0.x, v1.x, v2.x]) {
                    draw_scanline(v0.x, v0.z, v1.x, v1.z, z_buffer);
                    draw_scanline(v1.x, v1.z, v2.x, v2.z, z_buffer);
                    draw_scanline(v0.x, v0.z, v2.x, v2.z, z_buffer);
                    scanline_handler(origin, &mut z_buffer[range]);
                }
                return;
            } else {
                let frac = (yi + 1) as f32 - v0.y;
                side_x2 = fma![(v0.x) + dx02 * frac];
                side_z2 = fma![(v0.z) + dz02 * frac];

                if yi == v1yi {
                    let frac = (yi + 1) as f32 - v1.y;
                    side_x1 = fma![(v1.x) + dx12 * frac];
                    side_z1 = fma![(v1.z) + dz12 * frac];

                    if let Some((origin, range)) =
                        init_scanline(yi, z_buffer, &[v0.x, v1.x, side_x1, side_x2])
                    {
                        draw_scanline(v0.x, v0.z, v1.x, v1.z, z_buffer);
                        draw_scanline(v1.x, v1.z, side_x1, side_z1, z_buffer);
                        draw_scanline(v0.x, v0.z, side_x2, side_z2, z_buffer);
                        draw_scanline(side_x1, side_z1, side_x2, side_z2, z_buffer);
                        scanline_handler(origin, &mut z_buffer[range]);
                    }

                    yi += 1;
                    break 'top_half;
                } else {
                    side_x1 = fma![(v0.x) + dx01 * frac];
                    side_z1 = fma![(v0.z) + dz01 * frac];

                    if let Some((origin, range)) =
                        init_scanline(yi, z_buffer, &[v0.x, side_x1, side_x2])
                    {
                        draw_scanline(v0.x, v0.z, side_x1, side_z1, z_buffer);
                        draw_scanline(v0.x, v0.z, side_x2, side_z2, z_buffer);
                        draw_scanline(side_x1, side_z1, side_x2, side_z2, z_buffer);
                        scanline_handler(origin, &mut z_buffer[range]);
                    }
                }
            }
            yi += 1;
        } else if yi > v1yi {
            let frac = yi as f32 - v0.y;
            side_x2 = fma![(v0.x) + dx02 * frac];
            side_z2 = fma![(v0.z) + dz02 * frac];

            let frac = yi as f32 - v1.y;
            side_x1 = fma![(v1.x) + dx12 * frac];
            side_z1 = fma![(v1.z) + dz12 * frac];
            break 'top_half;
        } else {
            let frac = yi as f32 - v0.y;

            side_x2 = fma![(v0.x) + dx02 * frac];
            side_z2 = fma![(v0.z) + dz02 * frac];

            side_x1 = fma![(v0.x) + dx01 * frac];
            side_z1 = fma![(v0.z) + dz01 * frac];
        }

        while yi < min(v1yi, size.y as i32) {
            let next_side_x1 = side_x1 + dx01;
            let next_side_z1 = side_z1 + dz01;
            let next_side_x2 = side_x2 + dx02;
            let next_side_z2 = side_z2 + dz02;

            if let Some((origin, range)) = init_scanline(
                yi,
                z_buffer,
                &[next_side_x1, next_side_x2, side_x1, side_x2],
            ) {
                draw_scanline(
                    next_side_x1,
                    next_side_z1,
                    next_side_x2,
                    next_side_z2,
                    z_buffer,
                );
                draw_scanline(side_x1, side_z1, side_x2, side_z2, z_buffer);
                draw_scanline(side_x1, side_z1, next_side_x1, next_side_z1, z_buffer);
                draw_scanline(side_x2, side_z2, next_side_x2, next_side_z2, z_buffer);
                scanline_handler(origin, &mut z_buffer[range]);
            }

            side_x1 = next_side_x1;
            side_z1 = next_side_z1;
            side_x2 = next_side_x2;
            side_z2 = next_side_z2;

            yi += 1;
        }

        if yi >= size.y as i32 {
            return;
        }

        if yi == v1yi {
            if yi == v2yi {
                if let Some((origin, range)) =
                    init_scanline(yi, z_buffer, &[v2.x, v1.x, side_x1, side_x2])
                {
                    draw_scanline(v2.x, v2.z, v1.x, v1.z, z_buffer);
                    draw_scanline(v1.x, v1.z, side_x1, side_z1, z_buffer);
                    draw_scanline(v2.x, v2.z, side_x2, side_z2, z_buffer);
                    draw_scanline(side_x1, side_z1, side_x2, side_z2, z_buffer);
                    scanline_handler(origin, &mut z_buffer[range]);
                }
                return;
            } else {
                let frac = (yi + 1) as f32 - v1.y;
                let next_side_x1 = fma![(v1.x) + dx12 * frac];
                let next_side_z1 = fma![(v1.z) + dz12 * frac];
                let next_side_x2 = side_x2 + dx02;
                let next_side_z2 = side_z2 + dz02;

                if let Some((origin, range)) = init_scanline(
                    yi,
                    z_buffer,
                    &[next_side_x1, next_side_x2, side_x1, side_x2, v1.x],
                ) {
                    draw_scanline(
                        next_side_x1,
                        next_side_z1,
                        next_side_x2,
                        next_side_z2,
                        z_buffer,
                    );
                    draw_scanline(side_x1, side_z1, side_x2, side_z2, z_buffer);
                    draw_scanline(side_x1, side_z1, v1.x, v1.z, z_buffer);
                    draw_scanline(v1.x, v1.z, next_side_x1, next_side_z1, z_buffer);
                    draw_scanline(side_x2, side_z2, next_side_x2, next_side_z2, z_buffer);
                    scanline_handler(origin, &mut z_buffer[range]);
                }

                side_x1 = next_side_x1;
                side_z1 = next_side_z1;
                side_x2 = next_side_x2;
                side_z2 = next_side_z2;

                yi += 1;
            }
        } else {
            return;
        }

        break;
    } // 'top_half

    while yi < min(v2yi, size.y as i32) {
        let next_side_x1 = side_x1 + dx12;
        let next_side_z1 = side_z1 + dz12;
        let next_side_x2 = side_x2 + dx02;
        let next_side_z2 = side_z2 + dz02;

        if let Some((origin, range)) = init_scanline(
            yi,
            z_buffer,
            &[next_side_x1, next_side_x2, side_x1, side_x2],
        ) {
            draw_scanline(
                next_side_x1,
                next_side_z1,
                next_side_x2,
                next_side_z2,
                z_buffer,
            );
            draw_scanline(side_x1, side_z1, side_x2, side_z2, z_buffer);
            draw_scanline(side_x1, side_z1, next_side_x1, next_side_z1, z_buffer);
            draw_scanline(side_x2, side_z2, next_side_x2, next_side_z2, z_buffer);
            scanline_handler(origin, &mut z_buffer[range]);
        }

        side_x1 = next_side_x1;
        side_z1 = next_side_z1;
        side_x2 = next_side_x2;
        side_z2 = next_side_z2;

        yi += 1;
    }

    if yi >= size.y as i32 {
        return;
    }

    if yi == v2yi {
        if let Some((origin, range)) = init_scanline(yi, z_buffer, &[v2.x, side_x1, side_x2]) {
            draw_scanline(v2.x, v2.z, side_x1, side_z1, z_buffer);
            draw_scanline(v2.x, v2.z, side_x2, side_z2, z_buffer);
            draw_scanline(side_x1, side_z1, side_x2, side_z2, z_buffer);
            scanline_handler(origin, &mut z_buffer[range]);
        }
        return;
    }
}

fn init_scanline(
    y: i32,
    z_buffer: &mut [Range<f32>],
    x: &[f32],
) -> Option<(Point2<u32>, Range<usize>)> {
    let x_min = max(x.iter().map(|&x| x as i32).min().unwrap(), 0);
    let x_max = min(
        x.iter().map(|&x| x as i32).max().unwrap(),
        z_buffer.len() as i32 - 1,
    );
    if x_min <= x_max {
        let range = x_min as usize..x_max as usize + 1;
        for x in z_buffer.iter_mut() {
            *x = INFINITY..NEG_INFINITY;
        }
        Some((Point2::new(x_min as u32, y as u32), range))
    } else {
        None
    }
}

fn draw_scanline(x0: f32, z0: f32, x1: f32, z1: f32, z_buffer: &mut [Range<f32>]) {
    let (x0, z0, x1, z1) = if x1 < x0 {
        (x1, z1, x0, z0)
    } else {
        (x0, z0, x1, z1)
    };

    let v0xi = x0 as i32;
    let v1xi = x1 as i32;

    if max(v0xi, 0) > min(v1xi, z_buffer.len() as i32 - 1) {
        return;
    }

    let mut x;
    let mut z = z0;
    let dz = (z1 - z0) / (x1 - x0);

    if v0xi >= 0 {
        if v1xi == v0xi {
            let out = &mut z_buffer[v0xi as usize];
            out.start = [out.start, [z0, z1].fmin()].fmin();
            out.end = [out.end, [z0, z1].fmax()].fmax();
            return;
        }

        fma![z += ((v0xi + 1) as f32 - x0) * dz];
        x = v0xi + 1;

        let out = &mut z_buffer[v0xi as usize];
        out.start = [out.start, [z0, z].fmin()].fmin();
        out.end = [out.end, [z0, z].fmax()].fmax();
    } else {
        fma![z -= x0 * dz];
        x = 0;
    }

    while (x as usize) < min(v1xi as usize, z_buffer.len()) {
        let next_z = z + dz;

        let out = &mut z_buffer[x as usize];
        out.start = [out.start, [next_z, z].fmin()].fmin();
        out.end = [out.end, [next_z, z].fmax()].fmax();

        z = next_z;
        x += 1;
    }

    if (x as usize) < z_buffer.len() && x == v1xi {
        let out = &mut z_buffer[v1xi as usize];
        out.start = [out.start, [z1, z].fmin()].fmin();
        out.end = [out.end, [z1, z].fmax()].fmax();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use array::Array3;
    use ndarray::Array2;

    #[test]
    fn tricrast_sanity() {
        let size = Vector2::new(16, 16);
        let mut z_buffer = vec![0.0..0.0; size.x as usize];

        let patterns = vec![
            [
                Point3::new(4.5, 2.5, 1.0),
                Point3::new(15.5, 11.5, 3.0),
                Point3::new(7.5, 14.5, 7.0),
            ],
            // degenerate: point
            [
                Point3::new(4.0, 4.0, 4.0),
                Point3::new(4.0, 4.0, 2.0),
                Point3::new(4.0, 4.0, 1.0),
            ],
            // degenerate: horz line
            [
                Point3::new(4.0, 4.0, 4.0),
                Point3::new(8.0, 4.0, 2.0),
                Point3::new(4.0, 4.0, 1.0),
            ],
            // degenerate: vert line
            [
                Point3::new(4.0, 4.0, 4.0),
                Point3::new(4.0, 6.0, 2.0),
                Point3::new(4.0, 8.0, 1.0),
            ],
            // degenerate: line
            [
                Point3::new(4.0, 4.0, 4.0),
                Point3::new(5.0, 6.0, 2.0),
                Point3::new(6.0, 8.0, 1.0),
            ],
            // 01 overlap
            [
                Point3::new(4.0, 4.0, 4.0),
                Point3::new(8.0, 4.5, 2.0),
                Point3::new(6.0, 6.0, 1.0),
            ],
            // 23 overlap
            [
                Point3::new(4.0, 4.0, 4.0),
                Point3::new(8.0, 5.5, 2.0),
                Point3::new(6.0, 5.9, 1.0),
            ],
            // 012 overlap
            [
                Point3::new(4.0, 4.0, 4.0),
                Point3::new(8.0, 4.2, 2.0),
                Point3::new(6.0, 4.8, 1.0),
            ],
            // -X clip
            [
                Point3::new(-4.5, 2.5, 1.0),
                Point3::new(15.5, 11.5, 3.0),
                Point3::new(-7.5, 14.5, 7.0),
            ],
            // X clip
            [
                Point3::new(4.5, 2.5, 1.0),
                Point3::new(20.5, 11.5, 3.0),
                Point3::new(7.5, 14.5, 7.0),
            ],
            // -Y clip
            [
                Point3::new(4.5, -4.5, 1.0),
                Point3::new(15.5, -1.5, 3.0),
                Point3::new(7.5, 14.5, 7.0),
            ],
            // Clipped completey
            [
                Point3::new(4.5, -7.5, 1.0),
                Point3::new(15.5, -6.5, 3.0),
                Point3::new(7.5, -3.5, 7.0),
            ],
            // 0 is on `size.y`
            [
                Point3::new(4.5, 16.0, 1.0),
                Point3::new(15.5, 18.0, 3.0),
                Point3::new(7.5, 20.0, 7.0),
            ],
            // 1 is on `size.y`
            [
                Point3::new(4.5, 14.0, 1.0),
                Point3::new(15.5, 16.0, 3.0),
                Point3::new(7.5, 18.0, 7.0),
            ],
            // 2 is on `size.y`
            [
                Point3::new(4.5, 12.0, 1.0),
                Point3::new(15.5, 14.0, 3.0),
                Point3::new(7.5, 16.0, 7.0),
            ],
            // Other
            [
                Point3::new(6.387413, 22.655037, 10.262598),
                Point3::new(34.602814, 3.326641, 33.970768),
                Point3::new(13.196243, -2.00811, 8.0031395),
            ],
        ];

        for vertices in patterns {
            let mut z_predicted: Array2<Option<Range<f32>>> =
                Array2::default([size.y as usize, size.x as usize]);
            let mut visited: Array2<u8> = Array2::default([size.y as usize, size.x as usize]);
            let mut z_visited: Array2<Option<Range<f32>>> =
                Array2::default([size.y as usize, size.x as usize]);

            dbg!(vertices);

            let z_min = vertices.map(|v| v.z).fmin();
            let z_max = vertices.map(|v| v.z).fmax();

            tricrast(vertices, size, &mut z_buffer, |origin, z_buffer| {
                dbg!((origin, &z_buffer));
                for (x, z_range) in (origin.x..).zip(z_buffer.iter()) {
                    z_predicted[[origin.y as usize, x as usize]] = Some(z_range.clone());
                    visited[[origin.y as usize, x as usize]] |= 0b11;
                    assert!(z_range.start >= z_min - 0.0001);
                    assert!(z_range.end <= z_max + 0.0001);
                }
            });

            for x in 1..=49 {
                for y in 1..=49 {
                    let bx = x as f32 / 50.0;
                    let by = y as f32 / 50.0;
                    let p = vertices[0] + (vertices[1] - vertices[0]) * bx;
                    let p = p + (vertices[2] - p) * by;

                    let pi = p.cast::<isize>().unwrap();
                    for ix in pi.x - 1..=pi.x + 1 {
                        for iy in pi.y - 1..=pi.y + 1 {
                            if let Some(v) = visited.get_mut([iy as usize, ix as usize]) {
                                *v &= 0b1;
                            }
                            if let Some(v) = z_visited.get_mut([iy as usize, ix as usize]) {
                                let r = v.clone().unwrap_or(10000.0..-10000.0);
                                *v = Some([r.start, p.z].fmin()..[r.end, p.z].fmax());
                            }
                        }
                    }

                    if p.x < 0.0 || p.y < 0.0 || p.x >= size.x as f32 || p.y >= size.y as f32 {
                        continue;
                    }
                    let pi = p.cast::<usize>().unwrap();
                    let z_range = z_predicted[[pi.y, pi.x]].clone().unwrap_or(0.0..0.0);
                    assert!(
                        p.z >= z_range.start - 0.0001 && p.z <= z_range.end + 0.0001,
                        "{:?}.z is not in {:?}.\nMap: {:?}",
                        p,
                        z_predicted[[pi.y, pi.x]],
                        &z_predicted,
                    );
                }
            }

            for (i, v) in visited.indexed_iter() {
                if *v == 0b11 {
                    panic!("overconservativism: {:?}.\nMap: {:#?}", i, &visited);
                } else {
                    let row_z_visited = &z_visited[i];
                    let row_z_predicted = &z_predicted[i];
                    if let (Some(visited), Some(predicted)) = (row_z_visited, row_z_predicted) {
                        if predicted.start < visited.start - 0.5
                            || predicted.end > visited.end + 0.5
                        {
                            panic!(
                                "overconservativism: {:?} (pred = {:?}, actual = {:?}).\nMap: {:?}",
                                i, predicted, visited, &z_predicted
                            );
                        }
                    }
                }
            }
        }
    }
}
