//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate zangfx_common;
use zangfx_common::BinaryUInteger;

#[test]
fn is_power_of_two() {
    assert!(!(&0u32).is_power_of_two(), "0");
    assert!((&1u32).is_power_of_two(), "1");
    assert!((&2u32).is_power_of_two(), "2");
    assert!(!(&3u32).is_power_of_two(), "3");
}
