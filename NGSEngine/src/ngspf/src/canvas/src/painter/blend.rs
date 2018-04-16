//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use raduga::{prelude::*, SimdMode};
use rgb::RGBA;

mod table {
    // sRGB transfer function table. Generated by `build.rs`.
    include!(concat!(env!("OUT_DIR"), "/blend_table.rs"));
}

pub type Srgb8InternalColor = [u16; 4];
pub type Srgb8PremulInternalColor = [u16; 4];

/// Construct a `Srgb8InternalColor` from a color (with direct alpha).
pub fn srgb8_color_to_internal(x: RGBA<f32>) -> Srgb8InternalColor {
    let x_pm: RGBA<f32> = x.map_rgb(|v| v * x.a * (255.0 / 256.0));
    [
        // RGB [0, 4095 * 255 / 256]
        (x_pm.r * 4095.0 + 0.5) as u16,
        (x_pm.g * 4095.0 + 0.5) as u16,
        (x_pm.b * 4095.0 + 0.5) as u16,
        // Alpha [0, 32768]
        (x_pm.a * 32768.0 + 0.5) as u16,
    ]
}

/// Construct a `Srgb8PremulInternalColor` from a color (with direct alpha).
pub fn srgb8_premul_color_to_internal(x: RGBA<f32>) -> Srgb8PremulInternalColor {
    let x_pm: RGBA<f32> = x.map_rgb(|v| v * x.a);
    [
        // RGB [0, 4095 * 255 / 256]
        (x_pm.r * 4095.0 + 0.5) as u16,
        (x_pm.g * 4095.0 + 0.5) as u16,
        (x_pm.b * 4095.0 + 0.5) as u16,
        // Alpha [0, 32768]
        (x_pm.a * 32768.0 + 0.5) as u16,
    ]
}

/// Construct a `Srgb8InternalColor` by multiplying the alpha value of a given
/// `Srgb8InternalColor` by `coverage`.
pub fn srgb8_internal_mask(x: Srgb8InternalColor, coverage: u8) -> Srgb8InternalColor {
    // Map `[0, 255]` to `[0, 65536]`
    let cov = (coverage as u32) * 0x101 + (coverage >> 7) as u32;

    // Multiply each component by the coverage
    let f = |x: u16| ((x as u32 * cov) >> 16) as u16;
    [f(x[0]), f(x[1]), f(x[2]), f(x[3])]
}

/// Blend `src` over `dst`.
#[inline]
pub fn srgb8_alpha_over<M: SimdMode>(src: Srgb8InternalColor, dst: [M::U8; 4]) -> [M::U8; 4] {
    // [0, 255], alpha
    let dst_alpha = dst[3];

    // Convert to linear
    let dst_lin: [M::U16; 3] = unsafe {
        [
            // [0, 4095 * 2], linear color
            table::DECODE_SRGB.gather32_unchecked(dst[0].as_u32(), 1),
            table::DECODE_SRGB.gather32_unchecked(dst[1].as_u32(), 1),
            table::DECODE_SRGB.gather32_unchecked(dst[2].as_u32(), 1),
        ]
    };

    // Pre-multiply alpha
    let dst_alpha_7p = dst_alpha.as_i16() << 7;
    let dst_pm = [
        // [0, 4095 * 2 * 255 / 256], linear color (premul by `dst_alpha`)
        dst_lin[0].as_i16().mul_hrs_epi16(dst_alpha_7p).as_u16(),
        dst_lin[1].as_i16().mul_hrs_epi16(dst_alpha_7p).as_u16(),
        dst_lin[2].as_i16().mul_hrs_epi16(dst_alpha_7p).as_u16(),
    ];

    // Multiply the destination factor
    let dst_factor_p1 = M::I16::splat(((32768 - src[3]) >> 1) as i16);
    let dst_pm = [
        // [0, 4095 * 255 / 256], linear color (premul by `dst_alpha`)
        dst_pm[0].as_i16().mul_hrs_epi16(dst_factor_p1).as_u16(),
        dst_pm[1].as_i16().mul_hrs_epi16(dst_factor_p1).as_u16(),
        dst_pm[2].as_i16().mul_hrs_epi16(dst_factor_p1).as_u16(),
    ];

    // [0, 4080]
    let dst_factor_p3 = M::I16::splat(((32768 - src[3]) >> 3) as i16);
    let dst_alpha = dst_alpha_7p.as_i16().mul_hrs_epi16(dst_factor_p3).as_u16();

    // Combine
    let out_pm = [
        // [0, 4095 * 255 / 256], linear color (premul by `out_alpha`)
        dst_pm[0] + M::U16::splat(src[0]),
        dst_pm[1] + M::U16::splat(src[1]),
        dst_pm[2] + M::U16::splat(src[2]),
    ];
    // [0, 4080]
    let src_factor = src[3] as u32;
    let out_alpha = dst_alpha + M::U16::splat(((255 * src_factor + 1024) >> 11) as _);

    // De-pre-multiply alpha
    let depm_values: M::U32 = unsafe { table::DIV_4096.gather32_unchecked(out_alpha.as_u32(), 1) };
    let depm_mantissa = depm_values.as_u16();
    let depm_exponent = depm_values >> 16;

    // Better to keep `depm_exponent` as `u32` since AVX2 includes
    // variable left shift only for `epi32` (the `epi16` variant requires AVX512VL)

    let out_lin = [
        // [0, 4095], linear color
        out_pm[0].shl_var(depm_exponent).mul_hi_epu16(depm_mantissa),
        out_pm[1].shl_var(depm_exponent).mul_hi_epu16(depm_mantissa),
        out_pm[2].shl_var(depm_exponent).mul_hi_epu16(depm_mantissa),
    ];

    // Convert to sRGB
    unsafe {
        [
            table::ENCODE_SRGB.gather32_unchecked(out_lin[0].as_u32(), 1),
            table::ENCODE_SRGB.gather32_unchecked(out_lin[1].as_u32(), 1),
            table::ENCODE_SRGB.gather32_unchecked(out_lin[2].as_u32(), 1),
            (out_alpha >> 4).as_u8(),
        ]
    }
}

/// Blend `src` over `dst`.
#[inline]
pub fn srgb8_premul_alpha_over<M: SimdMode>(
    src: Srgb8PremulInternalColor,
    dst: [M::U8; 4],
) -> [M::U8; 4] {
    // [0, 255], alpha
    let dst_alpha = dst[3];

    // Convert to linear
    let dst_pm: [M::U16; 3] = unsafe {
        [
            // [0, 4095 * 2], linear color (premul by `dst_alpha`)
            table::DECODE_SRGB.gather32_unchecked(dst[0].as_u32(), 1),
            table::DECODE_SRGB.gather32_unchecked(dst[1].as_u32(), 1),
            table::DECODE_SRGB.gather32_unchecked(dst[2].as_u32(), 1),
        ]
    };

    // Multiply the destination factor
    let dst_factor_p1 = M::I16::splat(((32768 - src[3]) >> 1) as i16);
    let dst_pm = [
        // [0, 4095], linear color (premul by `dst_alpha`)
        dst_pm[0].as_i16().mul_hrs_epi16(dst_factor_p1).as_u16(),
        dst_pm[1].as_i16().mul_hrs_epi16(dst_factor_p1).as_u16(),
        dst_pm[2].as_i16().mul_hrs_epi16(dst_factor_p1).as_u16(),
    ];

    // [0, 4080]
    let dst_factor_p3 = M::I16::splat(((32768 - src[3]) >> 3) as i16);
    let dst_alpha_7p = dst_alpha.as_i16() << 7;
    let dst_alpha = dst_alpha_7p.mul_hrs_epi16(dst_factor_p3).as_u16();

    // Combine
    let out_lin = [
        // [0, 4095], linear color (premul by `out_alpha`)
        // possibly overflow
        dst_pm[0] + M::U16::splat(src[0]),
        dst_pm[1] + M::U16::splat(src[1]),
        dst_pm[2] + M::U16::splat(src[2]),
    ];
    // [0, 4080]
    let src_factor = src[3] as u32;
    let out_alpha = dst_alpha + M::U16::splat(((255 * src_factor + 1024) >> 11) as _);

    // Convert to sRGB
    unsafe {
        [
            table::ENCODE_SRGB.gather32_unchecked(out_lin[0].as_u32(), 1),
            table::ENCODE_SRGB.gather32_unchecked(out_lin[1].as_u32(), 1),
            table::ENCODE_SRGB.gather32_unchecked(out_lin[2].as_u32(), 1),
            (out_alpha >> 4).as_u8(),
        ]
    }
}

#[cfg(test)]
mod srgb8_tests {
    use super::*;

    // Reference (really slow) routines
    fn srgb_to_linear(x: f32) -> f32 {
        if x <= 0.04045 {
            x * (1.0 / 12.92)
        } else {
            ((x + 0.055) * (1.0 / 1.055)).powf(2.4)
        }
    }

    fn linear_to_srgb(x: f32) -> f32 {
        if x < 0.0031308 {
            12.92 * x.max(0.0)
        } else {
            1.055 * x.min(1.0).powf(0.41666) - 0.055
        }
    }

    fn alpha_over_premul(x: RGBA<f32>, y: RGBA<f32>) -> RGBA<f32> {
        RGBA::new(
            x.r * (1.0 - y.a) + y.r,
            x.g * (1.0 - y.a) + y.g,
            x.b * (1.0 - y.a) + y.b,
            x.a * (1.0 - y.a) + y.a,
        )
    }

    fn to_alpha_premul(x: RGBA<f32>) -> RGBA<f32> {
        RGBA::new(x.r * x.a, x.g * x.a, x.b * x.a, x.a)
    }

    fn from_alpha_premul(x: RGBA<f32>) -> RGBA<f32> {
        let factor = if x.a == 0.0 { 1.0 } else { x.a.recip() };
        RGBA::new(x.r * factor, x.g * factor, x.b * factor, x.a)
    }

    fn u8_approx_eq(a: [u8; 4], b: [u8; 4]) -> bool {
        a.iter()
            .zip(b.iter())
            .all(|(&x, &y)| ((x as i32) - (y as i32)).abs() <= 1)
    }

    fn u16_approx_eq(a: [u16; 4], b: [u16; 4]) -> bool {
        a.iter()
            .zip(b.iter())
            .all(|(&x, &y)| ((x as i32) - (y as i32)).abs() <= 4)
    }

    fn u8_to_rgba_f32(x: [u8; 4]) -> RGBA<f32> {
        RGBA::new(
            x[0] as f32 / 255.0,
            x[1] as f32 / 255.0,
            x[2] as f32 / 255.0,
            x[3] as f32 / 255.0,
        )
    }

    fn rgba_f32_to_u8(x: RGBA<f32>) -> [u8; 4] {
        [
            (x.r.min(1.0) * 255.0 + 0.5) as u8,
            (x.g.min(1.0) * 255.0 + 0.5) as u8,
            (x.b.min(1.0) * 255.0 + 0.5) as u8,
            (x.a.min(1.0) * 255.0 + 0.5) as u8,
        ]
    }

    #[test]
    fn internal_mask() {
        let a = srgb8_color_to_internal(RGBA::new(0.8, 0.6, 0.4, 1.0));
        let b = srgb8_color_to_internal(RGBA::new(0.8, 0.6, 0.4, 0.6));

        let a2 = srgb8_internal_mask(a, (0.6 / 1.0 * 255.0) as u8);

        assert!(u16_approx_eq(a2, b), "a2 ({:?}) != b ({:?})", a2, b);
    }

    #[test]
    fn alpha_over() {
        use raduga::ScalarMode;

        // Try many permutations
        let r_map: [u8; 4] = [0, 64, 192, 255];
        let g_map: [u8; 4] = [253, 1, 64, 192];
        let b_map: [u8; 4] = [150, 244, 9, 40];
        for pat in 0..0x100u32 {
            let dst_rgb = (pat & 0b11) as usize;
            let dst_a = (((pat >> 2) & 0b11) * 0b1010101) as u8;
            let dst = [r_map[dst_rgb], g_map[dst_rgb], b_map[dst_rgb], dst_a];

            let src_rgb = ((pat >> 4) & 0b11) as usize;
            let src_a = (((pat >> 6) & 0b11) * 0b1010101) as u8;
            let src = [r_map[src_rgb], g_map[src_rgb], b_map[src_rgb], src_a];

            let dst_f32 = u8_to_rgba_f32(dst).map_rgb(srgb_to_linear);
            let src_f32 = u8_to_rgba_f32(src).map_rgb(srgb_to_linear);

            let src_internal = srgb8_color_to_internal(src_f32);

            // fast version
            let out = srgb8_alpha_over::<ScalarMode>(src_internal, dst);

            // reference
            let out_f32 = from_alpha_premul(alpha_over_premul(
                to_alpha_premul(dst_f32),
                to_alpha_premul(src_f32),
            ));

            let out_f32_u8 = rgba_f32_to_u8(out_f32.map_rgb(linear_to_srgb));

            if out_f32.a > 0.0001 {
                assert!(
                    u8_approx_eq(out, out_f32_u8),
                    "out ({:?}) != out_f32_u8 ({:?}). (src = {:?}, dst = {:?})",
                    out,
                    out_f32_u8,
                    src,
                    dst
                );
            }
        }
    }

    #[test]
    fn premul_alpha_over() {
        use raduga::ScalarMode;

        // Try many permutations
        let r_map: [u8; 4] = [0, 64, 192, 255];
        let g_map: [u8; 4] = [253, 1, 64, 192];
        let b_map: [u8; 4] = [150, 244, 9, 40];
        for pat in 0..0x100u32 {
            let dst_rgb = (pat & 0b11) as usize;
            let dst_a = (((pat >> 2) & 0b11) * 0b1010101) as u8;
            let dst = [r_map[dst_rgb], g_map[dst_rgb], b_map[dst_rgb], dst_a];

            let src_rgb = ((pat >> 4) & 0b11) as usize;
            let src_a = (((pat >> 6) & 0b11) * 0b1010101) as u8;
            let src = [r_map[src_rgb], g_map[src_rgb], b_map[src_rgb], src_a];

            if src[3] == 0 {
                continue;
            }

            let dst_f32 = u8_to_rgba_f32(dst).map_rgb(srgb_to_linear);
            let src_f32 = u8_to_rgba_f32(src).map_rgb(srgb_to_linear);

            let src_internal = srgb8_premul_color_to_internal(from_alpha_premul(src_f32));

            // fast version
            let out = srgb8_premul_alpha_over::<ScalarMode>(src_internal, dst);

            // reference
            let out_f32 = alpha_over_premul(dst_f32, src_f32);

            let out_f32_u8 = rgba_f32_to_u8(out_f32.map_rgb(linear_to_srgb));

            if out_f32.a > 0.0001 {
                assert!(
                    u8_approx_eq(out, out_f32_u8),
                    "out ({:?}) != out_f32_u8 ({:?}). (src = {:?}, dst = {:?})",
                    out,
                    out_f32_u8,
                    src,
                    dst
                );
            }
        }
    }
}
