//! yFFT
//! ====
//!
//! Simple FFT library written purely in Rust. Requires a Nightly Rust compiler
//! for x86 intrinsics.
//!
//! License
//! -------
//!
//! Follows the license of the parent project (Nightingales).
//!

//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#![feature(test)]

extern crate num_complex;
extern crate num_iter;
extern crate num_traits;

use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};
use std::fmt::Debug;

mod setup;
mod kernel;
mod env;

pub trait Num :
    Clone + Debug + AddAssign + SubAssign + MulAssign + DivAssign + num_traits::Float + num_traits::FloatConst + num_traits::Zero + 'static {}
impl<T> Num for T where T :
    Clone + Debug + AddAssign + SubAssign + MulAssign + DivAssign + num_traits::Float + num_traits::FloatConst + num_traits::Zero + 'static {}

#[inline]
fn complex_from_slice<T : Num>(x: &[T]) -> num_complex::Complex<T> {
    num_complex::Complex::new(x[0], x[1])
}

pub use setup::{DataOrder, DataFormat, Options, Setup};
pub use env::Env;

extern crate test;

#[cfg(test)]
mod tests {
    use test::Bencher;
    use super::*;

    // To convert the result into a BenchFFT MFLOPS, use the following formula:
    //
    //   mflops = 5000 N log2(N) / (time for one FFT in nanoseconds)

    fn run_single_benchmark(size: usize, b: &mut Bencher) {
        let setup: Setup<f32> = Setup::new(&Options {
            input_data_order: DataOrder::Natural,
            output_data_order: DataOrder::Natural,
            input_data_format: DataFormat::Complex,
            output_data_format: DataFormat::Complex,
            len: size,
            inverse: false
        }).unwrap();
        let mut senv = Env::new(&setup);
        let mut buf = vec![0f32; size * 2];
        b.iter(move || {
            senv.transform(buf.as_mut_slice());
        })
    }

    #[bench] fn simple_benchmark_00001(b: &mut Bencher) { run_single_benchmark(1, b); }
    #[bench] fn simple_benchmark_00002(b: &mut Bencher) { run_single_benchmark(2, b); }
    #[bench] fn simple_benchmark_00004(b: &mut Bencher) { run_single_benchmark(4, b); }
    #[bench] fn simple_benchmark_00008(b: &mut Bencher) { run_single_benchmark(8, b); }
    #[bench] fn simple_benchmark_00016(b: &mut Bencher) { run_single_benchmark(16, b); }
    #[bench] fn simple_benchmark_00032(b: &mut Bencher) { run_single_benchmark(32, b); }
    #[bench] fn simple_benchmark_00064(b: &mut Bencher) { run_single_benchmark(64, b); }
    #[bench] fn simple_benchmark_00128(b: &mut Bencher) { run_single_benchmark(128, b); }
    #[bench] fn simple_benchmark_00256(b: &mut Bencher) { run_single_benchmark(256, b); }
    #[bench] fn simple_benchmark_00512(b: &mut Bencher) { run_single_benchmark(512, b); }
    #[bench] fn simple_benchmark_01024(b: &mut Bencher) { run_single_benchmark(1024, b); }
    #[bench] fn simple_benchmark_02048(b: &mut Bencher) { run_single_benchmark(2048, b); }
    #[bench] fn simple_benchmark_04096(b: &mut Bencher) { run_single_benchmark(4096, b); }
    #[bench] fn simple_benchmark_08192(b: &mut Bencher) { run_single_benchmark(8192, b); }
    #[bench] fn simple_benchmark_16384(b: &mut Bencher) { run_single_benchmark(16384, b); }

    // Following tests doesn't include the bit revesal pass, so the result cannot be compared
    // with other benchmarks

    fn run_dif_benchmark(size: usize, b: &mut Bencher) {
        let setup: Setup<f32> = Setup::new(&Options {
            input_data_order: DataOrder::Natural,
            output_data_order: DataOrder::Swizzled,
            input_data_format: DataFormat::Complex,
            output_data_format: DataFormat::Complex,
            len: size,
            inverse: false
        }).unwrap();
        let mut senv = Env::new(&setup);
        let mut buf = vec![0f32; size * 2];
        b.iter(move || {
            senv.transform(buf.as_mut_slice());
        })
    }

    #[bench] fn dif_benchmark_00064(b: &mut Bencher) { run_dif_benchmark(64, b); }
    #[bench] fn dif_benchmark_00256(b: &mut Bencher) { run_dif_benchmark(256, b); }
    #[bench] fn dif_benchmark_00512(b: &mut Bencher) { run_dif_benchmark(512, b); }
    #[bench] fn dif_benchmark_02048(b: &mut Bencher) { run_dif_benchmark(2048, b); }
    #[bench] fn dif_benchmark_08192(b: &mut Bencher) { run_dif_benchmark(8192, b); }

    fn run_dit_benchmark(size: usize, b: &mut Bencher) {
        let setup: Setup<f32> = Setup::new(&Options {
            input_data_order: DataOrder::Swizzled,
            output_data_order: DataOrder::Natural,
            input_data_format: DataFormat::Complex,
            output_data_format: DataFormat::Complex,
            len: size,
            inverse: false
        }).unwrap();
        let mut senv = Env::new(&setup);
        let mut buf = vec![0f32; size * 2];
        b.iter(move || {
            senv.transform(buf.as_mut_slice());
        })
    }

    #[bench] fn dit_benchmark_00064(b: &mut Bencher) { run_dit_benchmark(64, b); }
    #[bench] fn dit_benchmark_00256(b: &mut Bencher) { run_dit_benchmark(256, b); }
    #[bench] fn dit_benchmark_00512(b: &mut Bencher) { run_dit_benchmark(512, b); }
    #[bench] fn dit_benchmark_02048(b: &mut Bencher) { run_dit_benchmark(2048, b); }
    #[bench] fn dit_benchmark_08192(b: &mut Bencher) { run_dit_benchmark(8192, b); }
}
