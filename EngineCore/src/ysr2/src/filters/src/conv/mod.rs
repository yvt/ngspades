//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Non-uniform partitioned convolution filter.
use yfft;
use std::sync::Arc;

mod ir;
mod mixer;
mod source;

#[cfg(test)]
mod tests;

pub use self::ir::*;
pub use self::mixer::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvParams {
    /// The block sizes and the number of blocks used to perform non-uniform
    /// partitioned convolution.
    ///
    /// Each element indicates the log2 of the block size and the maximum number
    /// of blocks of a block group.
    ///
    /// The following conditions have to be met:
    ///
    ///  - `blocks.len() > 0`
    ///  - For every non-zero `i: size`, `blocks[i].1` must be greater than zero.
    ///  - Every block size must be less than `1 << 20`. In reality, such a
    ///    large block size would be useless due to the FFT's numerical error.
    ///  - The elements must be in a strict ascending order in regard to their
    ///    block sizes. In other words, for every `i: usize < blocks.len() - 1`,
    ///    `blocks[i].0 < blocks[i + 1].0` must be `true`.
    ///  - For every `i: size`,
    //     `offset[i] % (1 << blocks[i].0) == 0 && offset[i] > 0`
    ///    must be `true`, where
    ///
    ///    ```text
    ///    offset[i] = if i == 0 {
    ///        latency
    ///    } else {
    ///        offset[i - 1] + (blocks[i - 1].1 << blocks[i - 1].0)
    ///    }
    ///    ```
    ///  - `offset[i]: usize` must not overflow when computed using the above
    ///    definition.
    ///
    /// The meximum length of an impulse response is determined by
    /// `offset[blocks.len()] - latency` .
    pub blocks: Vec<(u32, usize)>,

    /// The input-to-output latency in samples.
    ///
    /// The minimum value is `1 << blocks[0].0` as implied by the restrictions
    /// of `blocks`.
    pub latency: usize,
}

#[derive(Debug, Clone)]
pub struct ConvSetup {
    params: ConvParams,
    groups: Vec<ConvGroupInfo>,
}

#[derive(Debug, Clone)]
struct ConvGroupInfo {
    fft_setup: [Arc<yfft::Setup<f32>>; 2],

    /// The length of frequency-domain delay line for the specified block size.
    ///
    /// Decremented by 1 if `use_tdr_on_mapping == true`.
    input_fdl_delay: usize,

    /// Indicates whether the TDR method is applied on the input (source)
    /// transform.
    use_tdr_on_source: bool,

    /// Indicates whether the TDR method is applied on the frquency domain
    /// operations.
    use_tdr_on_mapping: bool,
}

#[derive(Debug, Clone)]
struct ConvEnv {
    fft_envs: Vec<[yfft::Env<f32, Arc<yfft::Setup<f32>>>; 2]>,
}

impl ConvSetup {
    pub fn new(params: &ConvParams) -> Self {
        // Check the sanity of the `ConvParams`
        assert_ne!(params.blocks.len(), 0);

        for i in 0..params.blocks.len() {
            assert!(params.blocks[i].0 < 20);
        }

        for i in 0..params.blocks.len() - 1 {
            assert!(params.blocks[i].0 < params.blocks[i + 1].0);
            assert_ne!(params.blocks[i].1, 0);
        }

        let groups = {
            let mut position = params.latency;
            params
                .blocks
                .iter()
                .enumerate()
                .map(|(i, &(size_log2, num_blocks))| {
                    assert_eq!(position % (1 << size_log2), 0);
                    assert_ne!(position, 0);
                    let fdl_delay = (position >> size_log2) - 1;
                    if i < params.blocks.len() - 1 {
                        position = position.checked_add(num_blocks << size_log2).unwrap();
                    }

                    let use_tdr_on_source = fdl_delay > 0;
                    let use_tdr_on_mapping = fdl_delay > 1;

                    let mut input_fdl_delay = fdl_delay;

                    if use_tdr_on_mapping {
                        input_fdl_delay -= 1;
                    }

                    let len = 2 << size_log2;
                    let fft_setup = [
                        yfft::Setup::new(&yfft::Options {
                            input_data_order: yfft::DataOrder::Natural,
                            output_data_order: yfft::DataOrder::Natural,
                            input_data_format: yfft::DataFormat::Real,
                            output_data_format: yfft::DataFormat::HalfComplex,
                            len,
                            inverse: false,
                        }).map(Arc::new)
                            .unwrap(),
                        yfft::Setup::new(&yfft::Options {
                            input_data_order: yfft::DataOrder::Natural,
                            output_data_order: yfft::DataOrder::Natural,
                            input_data_format: yfft::DataFormat::HalfComplex,
                            output_data_format: yfft::DataFormat::Real,
                            len,
                            inverse: true,
                        }).map(Arc::new)
                            .unwrap(),
                    ];

                    ConvGroupInfo {
                        input_fdl_delay,
                        fft_setup,
                        use_tdr_on_source,
                        use_tdr_on_mapping,
                    }
                })
                .collect()
        };

        Self {
            params: params.clone(),
            groups,
        }
    }

    /// Retrieve the `ConvParams` this `ConvSetup` was created with.
    pub fn params(&self) -> &ConvParams {
        &self.params
    }

    fn group(&self, index: usize) -> &ConvGroupInfo {
        &self.groups[index]
    }
}

impl ConvEnv {
    pub fn new(setup: &ConvSetup) -> Self {
        Self {
            fft_envs: setup
                .groups
                .iter()
                .map(|group| {
                    [
                        yfft::Env::new(group.fft_setup[0].clone()),
                        yfft::Env::new(group.fft_setup[1].clone()),
                    ]
                })
                .collect(),
        }
    }
}
