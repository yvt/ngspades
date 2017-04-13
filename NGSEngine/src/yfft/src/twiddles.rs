//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use num_complex::Complex;
use num_traits::Zero;
use super::Num;

#[derive(Debug, Clone)]
pub struct TwiddlesTable<T> {
    twiddles: Vec<Complex<T>>,
    indices: Vec<usize>,
    radixes: Vec<usize>,
}

type TwiddlesTable32 = TwiddlesTable<f32>;
type TwiddlesTable64 = TwiddlesTable<f64>;

pub fn factorize(mut x: usize) -> Result<Vec<usize>, ()> {

    let mut vec = Vec::new();

    while x > 1 {
        let radix =
            /* if x % 4 == 0 {
                4
            } else */ if x % 2 == 0 {
                2
            } else {
                return Err(())
            };
        vec.push(radix);
        x /= radix;
    }
    Ok(vec)
}

impl<T: Num> TwiddlesTable<T> {
    pub fn new(size: usize) -> Result<Self, ()> {
        let mut table = Self {
            twiddles: Vec::new(),
            indices: Vec::new(),
            radixes: try!(factorize(size))
        };

        let delta1 = T::from(2).unwrap() * T::PI() / T::from(size).unwrap();
        let mut q: usize = 1;

        {
            let ref radixes = table.radixes;

            for radix_ref in radixes {
                let radix = *radix_ref;
                table.indices.push(table.twiddles.len());
                let old_q = q;
                q *= radix;
                for i in 1 .. radix {
                    let delta = delta1 * T::from(i * old_q).unwrap();
                    let mut angle = delta;
                    let mut j = 0;
                    while j < size {
                        let v: Complex<T> = Complex::new(Zero::zero(), angle).exp();
                        table.twiddles.push(v);

                        j += q; angle += delta;
                    }
                }
            }
        }

        table.indices.push(table.twiddles.len());

        assert_eq!(table.twiddles.len(), size - 1);

        Ok(table)
    }

    pub fn radixes(&self) -> &[usize] {
        self.radixes.as_slice()
    }

    pub fn twiddles(&self, level: usize) -> &[Complex<T>] {
        let idx1 = self.indices[level];
        let idx2 = self.indices[level + 1];
        &self.twiddles[idx1 .. idx2]
    }

}

#[cfg(test)]
mod tests {
    use super::{TwiddlesTable32, TwiddlesTable64};
    #[test]
    fn create_twiddles() {
        TwiddlesTable32::new(1).unwrap();
        TwiddlesTable32::new(2).unwrap();
        TwiddlesTable32::new(256).unwrap();
        TwiddlesTable64::new(1).unwrap();
        TwiddlesTable64::new(2).unwrap();
        TwiddlesTable64::new(256).unwrap();
    }
}

