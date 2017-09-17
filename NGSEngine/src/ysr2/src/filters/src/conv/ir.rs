//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use yfft;
use conv::ConvSetup;

/// Processed single-channel impulse response data.
///
/// `IrSpectrum`s are created for a specific `ConvSetup` and must be used with
/// filters with the same or a compatible `ConvSetup`,
#[derive(Debug, Clone)]
pub struct IrSpectrum {
    blocks: Vec<Vec<Vec<f32>>>,
}

impl IrSpectrum {
    /// Constructs an `IrSpectrum` from a given impulse response.
    pub fn from_ir(ir: &[f32], setup: &ConvSetup) -> Self {
        let params = setup.params();
        let mut buffer = vec![0.0; 2 << params.blocks.last().unwrap().0];

        let mut start = 0;
        let blocks = params
            .blocks
            .iter()
            .enumerate()
            .map(|(i, &(size_log2, max_count))| {
                let mut blocks = Vec::new();
                let size = 1 << size_log2;
                let ref setup = setup.group(i).fft_setup[0];
                let mut env = yfft::Env::new(&**setup);
                let factor = 1.0 / size as f32;
                while start < ir.len() && blocks.len() < max_count {
                    let mut left = ir.len() - start;
                    if left > size {
                        left = size;
                    }

                    buffer[0..left].copy_from_slice(&ir[start..start + left]);
                    for x in buffer[0..left].iter_mut() {
                        *x *= factor;
                    }
                    for x in buffer[left..size * 2].iter_mut() {
                        *x = 0.0;
                    }

                    env.transform(&mut buffer[0..size * 2]);
                    blocks.push(Vec::from(&buffer[0..size * 2]));

                    start += left;
                }
                blocks
            })
            .collect();

        Self { blocks }
    }

    /// Retrieve the number of blocks of the specified size.
    ///
    /// `size_index` specifies an index into `ConvParams::blocks`. The returned
    /// value is bounded by `blocks[size_index].1` except for the largest blocks.
    pub fn num_blocks_for_size(&self, size_index: usize) -> usize {
        self.blocks[size_index].len()
    }

    /// Retrieve a slice for the specified block, in the `HalfComplex` format.
    ///
    /// For a `IrSpectrum` created with a `ConvSetup` created with a `ConvParams`
    /// `params`, this returns a slice of the length `2 << params.blocks[size_index].0`.
    pub fn get(&self, size_index: usize, index: usize) -> &[f32] {
        &self.blocks[size_index][index][..]
    }
}
