//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use num_complex::Complex;
use std::result::Result;
use super::Num;
use super::twiddles::{TwiddlesTable, factorize};
use super::kernel::{Kernel, KernelType, KernelCreationParams, new_bit_reversal_kernel};

#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum DataOrder {
    /// The data is ordered in a natural order.
    Natural,

    /// The data is ordered in a bit-reversal order.
    Swizzled
}


#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub enum DataFormat {
    Complex,
    Real
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Options {
    pub input_data_order: DataOrder,
    pub output_data_order: DataOrder,
    pub input_data_format: DataFormat,
    pub output_data_format: DataFormat,
    pub len: usize,
    pub inverse: bool
}

#[derive(Debug)]
pub struct Setup<T> {
    // #[doc(hidden)]
    // pub twiddles_table: TwiddlesTable<T>,
    #[doc(hidden)]
    pub kernels: Vec<Box<Kernel<T>>>
}

impl<T> Setup<T> where T : Num {
    pub fn new(options: &Options) -> Result<Self, ()> {
        if options.len == 0 {
            return Err(())
        }

        let (post_bit_reversal, kernel_type) =
            match (options.input_data_order, options.output_data_order) {
                (DataOrder::Natural,  DataOrder::Natural)  => (true,  KernelType::Dif),
                (DataOrder::Swizzled, DataOrder::Natural)  => (false, KernelType::Dit),
                (DataOrder::Natural,  DataOrder::Swizzled) => (false, KernelType::Dif),
                (DataOrder::Swizzled, DataOrder::Swizzled) => return Err(())
            };

        match (options.input_data_format, options.output_data_format, options.inverse) {
            (DataFormat::Complex, DataFormat::Complex, _) => {},
            (DataFormat::Real, DataFormat::Complex, false) => unimplemented!(),
            (DataFormat::Complex, DataFormat::Real, true) => unimplemented!(),
            _ => return Err(())
        }

        let mut radixes = try!(factorize(options.len));
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
            KernelType::Dit => unimplemented!()
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
