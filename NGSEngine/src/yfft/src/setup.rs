//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use std::result::Result;
use std::error;
use std::fmt;
use super::Num;
use super::kernel::{Kernel, KernelType, KernelCreationParams, new_bit_reversal_kernel};

/// Specifies the data order in which the data is supplied to or returned from the kernel.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum DataOrder {
    /// The data is ordered in a natural order.
    Natural,

    /// The data is ordered in a bit-reversal order with arbitrary radixes.
    /// Use this value if you intend to process the output in an order-independent way and transform it back to the
    /// natural order.
    Swizzled,

    /// The data is ordered in a Radix-2 bit-reversal order.
    /// The data length must be a power of two.
    BitReversed
}

/// Specifies the data format.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum DataFormat {
    /// Specifies the interleaved complex format.
    Complex,

    /// Not supported.
    Real
}

/// The FFT kernel configuration.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Options {
    /// Specifies the input data order.
    ///
    /// Must be `Natural` if `output_data_order` is not `Natural`, or put in another way, this and `input_data_order`
    /// must not be `Naterual` at the same time.
    pub input_data_order: DataOrder,

    /// Specifies the output data order.
    ///
    /// Must be `Natural` if `input_data_order` is not `Natural`, or put in another way, this and `output_data_order`
    /// must not be `Naterual` at the same time.
    pub output_data_order: DataOrder,

    /// Specifies the input data format.
    pub input_data_format: DataFormat,

    /// Specifies the output data format.
    pub output_data_format: DataFormat,

    /// Specifies the length of the data to be processed.
    pub len: usize,

    /// Specifies whether the inverse (backward) transformation is used.
    pub inverse: bool
}

/// The error type which is returned from the `Setup` creation function.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum PlanError {
    /// A parameter was incorrect.
    InvalidInput
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PlanError::InvalidInput => write!(f, "The parameter is invalid.")
        }
    }
}

impl error::Error for PlanError {
    fn description(&self) -> &str {
        match *self {
            PlanError::InvalidInput => "Invalid input"
        }
    }

    fn cause(&self) -> Option<&error::Error> { None }
}

/// Encapsulates the FFT kernel configuration.
#[derive(Debug)]
pub struct Setup<T> {
    #[doc(hidden)]
    pub kernels: Vec<Box<Kernel<T>>>
}

pub fn factorize_radix2(x: usize) -> Result<Vec<usize>, PlanError> {
    if (x & (x - 1)) == 0 {
        Ok(vec![2; x.trailing_zeros() as usize])
    } else {
        Err(PlanError::InvalidInput)
    }
}

pub fn factorize(mut x: usize) -> Vec<usize> {
    let mut vec = Vec::new();
    let mut possible_factor_min = 3;

    while x > 1 {
        let radix =
            if x % 4 == 0 && false {
                4
            } else if x % 2 == 0 {
                2
            } else {
                let found_radix = (0..)
                    .map(|r| r * 2 + possible_factor_min)
                    .filter(|r| x % r == 0)
                    .nth(0)
                    .unwrap();
                possible_factor_min = found_radix;
                found_radix
            };
        vec.push(radix);
        x /= radix;
    }

    vec
}

impl<T> Setup<T> where T : Num + 'static {
    pub fn new(options: &Options) -> Result<Self, PlanError> {
        if options.len == 0 {
            return Err(PlanError::InvalidInput)
        }

        let constain_radix2 =
            options.input_data_order == DataOrder::BitReversed ||
            options.output_data_order == DataOrder::BitReversed;

        let input_swizzled = match options.input_data_order {
            DataOrder::Natural => false,
            DataOrder::Swizzled => true,
            DataOrder::BitReversed => true
        };

        let output_swizzled = match options.output_data_order {
            DataOrder::Natural => false,
            DataOrder::Swizzled => true,
            DataOrder::BitReversed => true
        };

        let (post_bit_reversal, kernel_type) =
            match (input_swizzled, output_swizzled) {
                (false, false) => (true,  KernelType::Dif),
                (true,  false) => (false, KernelType::Dit),
                (false, true)  => (false, KernelType::Dif),
                (true,  true)  => return Err(PlanError::InvalidInput)
            };

        match (options.input_data_format, options.output_data_format, options.inverse) {
            (DataFormat::Complex, DataFormat::Complex, _) => {},
            (DataFormat::Real, DataFormat::Complex, false) => unimplemented!(),
            (DataFormat::Complex, DataFormat::Real, true) => unimplemented!(),
            _ => return Err(PlanError::InvalidInput)
        }

        let mut radixes = if constain_radix2 {
            try!(factorize_radix2(options.len))
        } else {
            factorize(options.len)
        };
        if kernel_type == KernelType::Dit {
            radixes.reverse();
        }

        let mut kernels = Vec::new();
        match kernel_type {
            KernelType::Dif => {
                let mut unit = options.len;
                for radix_ref in &radixes {
                    let radix = *radix_ref;
                    unit /= radix;
                    kernels.push(Kernel::new(&KernelCreationParams {
                        size: options.len,
                        kernel_type: kernel_type,
                        radix: radix,
                        unit: unit,
                        inverse: options.inverse,
                    }));
                }
            },
            KernelType::Dit => {
                let mut unit = 1;
                for radix_ref in &radixes {
                    let radix = *radix_ref;
                    kernels.push(Kernel::new(&KernelCreationParams {
                        size: options.len,
                        kernel_type: kernel_type,
                        radix: radix,
                        unit: unit,
                        inverse: options.inverse,
                    }));
                    unit *= radix;
                }
            },
        }

        if post_bit_reversal && options.len > 1 {
            kernels.push(new_bit_reversal_kernel(radixes.as_slice()));
        }

        Ok(Self {
            kernels: kernels
        })
    }

    #[doc(hidden)]
    pub fn required_work_area_size(&self) -> usize {
        self.kernels.iter()
            .map(|k| k.required_work_area_size())
            .max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factorize() {
        assert_eq!(factorize(2), vec![2]);
    }

    #[test]
    fn test_factorize_radix2() {
        assert_eq!(factorize_radix2(4), Ok(vec![2, 2]));
        assert_eq!(factorize_radix2(5), Err(PlanError::InvalidInput));
    }
}
