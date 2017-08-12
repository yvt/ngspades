//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate ysr2_common;
extern crate ysr2_mixer;
use ysr2_common::stream::{StreamProperties, ChannelConfig};
use ysr2_mixer::clip::Clip;
use ysr2_mixer::clipplayer::ClipPlayer;

#[test]
fn bit_exact() {
    let prop = StreamProperties {
        sampling_rate: 44100f64,
        num_channels: 1,
        channel_config: ChannelConfig::Monaural,
    };
    let data: Vec<f32> = (1..64i32).map(|x| x as f32).collect();
    let clip = Clip::new(data.len(), None, &prop);
    {
        let mut writer = clip.write_samples();
        writer.get_channel_mut(0).copy_from_slice(&data);
    }

    let mut player = ClipPlayer::new(&clip, &prop);
    let mut buffer = vec![0f32; 128];
    player.render(&mut [&mut buffer]);
    println!("clip: {:?}", data);
    println!("rendered: {:?}", buffer);
    for (i, &value) in data.iter().enumerate() {
        assert_eq!(value, buffer[i + 2]);
    }
}
