//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[derive(Debug, PartialEq, Clone)]
pub struct StreamProperties {
    pub sampling_rate: f64,
    pub num_channels: usize,
    pub channel_config: ChannelConfig,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum ChannelConfig {
    /// The monaural configuration.
    ///
    /// The number of channel must be one.
    Monaural,

    /// The stereo configuration.
    ///
    /// The number of channel must be two.
    Stereo,

    /// Generic configuration.
    Generic,
}
