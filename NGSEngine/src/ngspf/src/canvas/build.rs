//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

fn write_generated_code(name: &str, code: &str) {
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join(name);
    let mut f = File::create(&dest_path).unwrap();
    f.write_all(code.as_bytes()).unwrap();
}

fn main() {
    use std::fmt::Write;
    let mut table_src = String::new();

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

    writeln!(&mut table_src, "pub static DECODE_SRGB: [u16; 256] = [").unwrap();
    for i in 0..256 {
        write!(
            &mut table_src,
            "{}, ",
            (srgb_to_linear(i as f32 / 255.0) * 8190.0 + 0.5) as u16
        ).unwrap();
    }
    writeln!(&mut table_src, "];").unwrap();

    writeln!(&mut table_src, "pub static ENCODE_SRGB: [u8; 4097] = [").unwrap();
    for i in 0..4097 {
        write!(
            &mut table_src,
            "{}, ",
            (linear_to_srgb(i as f32 / 4095.0) * 255.0 + 0.5) as u8
        ).unwrap();
    }
    writeln!(&mut table_src, "];").unwrap();

    writeln!(&mut table_src, "pub static DIV_4096: [u32; 4096] = [").unwrap();
    for i in 0..4096 {
        let factor: u32 = 65536 * 4096 / if i == 0 { 1 } else { i };
        let exponent = 16u32.saturating_sub(factor.leading_zeros());
        let mantissa = factor >> exponent;
        assert!(
            mantissa < 0x10000 && mantissa >= 0x8000,
            "{:?}",
            (i, mantissa)
        );
        write!(&mut table_src, "0x{:08x}, ", mantissa | (exponent << 16)).unwrap();
    }
    writeln!(&mut table_src, "];").unwrap();

    write_generated_code("blend_table.rs", &table_src);
}
