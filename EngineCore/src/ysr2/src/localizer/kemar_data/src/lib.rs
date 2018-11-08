//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! HRTF Measurement Data of a KEMAR Dummy-Head Microphone
//! ======================================================
//!
//! Credits
//! -------
//!
//! This data is Copyright 1994 by the MIT Media Laboratory. It is provided free
//! with no restrictions on use, provided the authors are cited when the data is
//! used in any research or commercial application.
//!
//! ```text
//! Bill Gardner <billg@media.mit.edu>
//! Keith Martin <kdm@media.mit.edu>
//! MIT Media Lab Machine Listening Group
//! May 18, 1994
//! ```
use std::fmt;

/// A set of response data at a specific elevation.
#[derive(Debug)]
pub struct Ring {
    /// The elevation measured in degrees in the range `[-40, 90]`, in step of
    /// 10 degrees.
    pub elevation: i32,

    /// Samples the this elevation.
    ///
    /// - For the elevation `90`, this contains exactly one `Sample`.
    /// - For other elevations, this contains `Sample`s with `azimuth` in the
    ///   range `[0, 180]`, with a varying number of elements, sorted by
    ///   `azimuth`.
    ///
    /// Note: The elevation `50` does not have a sample with `azimuth == 180`.
    pub samples: &'static [Sample],
}

/// A response data at a specific direction.
///
/// Each response data contains two impulse responses of 128 elements for
/// each of the left and right ear.
#[repr(C)]
#[repr(align(16))]
pub struct Sample {
    /// The azimuth measured in degrees, in the range `[0, 180]`.
    pub azimuth: i32,

    _pad: [u32; 3],

    /// DFT-ed impulse response for each of the left and right ear, in the
    /// `yfft::DataFormat::HalfComplex` format. The IR is zero-padded before
    /// DFT is applied and is scaled by 1/256 so it is handy to use for
    /// convolution.
    ///
    /// This array is 16-byte aligned.
    pub ir_fft_hc: [[f32; 256]; 2],
}

impl fmt::Debug for Sample {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Sample")
            .field("azimuth", &self.azimuth)
            .finish()
    }
}

/// A set of HRTF response data.
pub static KEMAR_DATA: &[Ring; 14] = &KEMAR_DATA_INTERNAL;

include!(concat!(env!("OUT_DIR"), "/data.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_alignment() {
        for ring in KEMAR_DATA.iter() {
            for sample in ring.samples.iter() {
                for (i, ch) in sample.ir_fft_hc.iter().enumerate() {
                    let p = ch.as_ptr() as usize;
                    assert!((p & 15) == 0, "unaligned ir_fft_hc found at {:?}", (
                        ring.elevation,
                        sample.azimuth,
                        i,
                    ))
                }
            }
        }
    }
}
