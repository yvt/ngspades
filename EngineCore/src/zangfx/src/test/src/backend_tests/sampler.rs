//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::TestDriver;

pub fn sampler_create<T: TestDriver>(driver: T) {
    driver.for_each_device(&mut |device| {
        let mut builder = device.build_sampler();
        builder.build().unwrap();
    });
}
