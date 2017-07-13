//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use wsi_core;
use ash::vk;

pub fn translate_color_space(value: wsi_core::ColorSpace) -> Option<vk::ColorSpaceKHR> {
    match value {
        wsi_core::ColorSpace::SrgbNonlinear => Some(vk::ColorSpaceKHR::SrgbNonlinear),
    }
}

pub fn reverse_translate_color_space(value: vk::ColorSpaceKHR) -> Option<wsi_core::ColorSpace> {
    match value {
        vk::ColorSpaceKHR::SrgbNonlinear => Some(wsi_core::ColorSpace::SrgbNonlinear),
        // _ => None,
    }
}
