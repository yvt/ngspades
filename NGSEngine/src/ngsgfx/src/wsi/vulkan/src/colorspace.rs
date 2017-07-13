//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;
use wsi_core;
use ash::vk;
use backend_vulkan::imp::{reverse_translate_image_format, translate_image_format};

use std::collections::{HashSet, HashMap};
use std::iter::FromIterator;
use thunk::{Thunk, LazyRef};

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

pub(crate) fn choose_visual<S, T>(
    candidates: &[(Option<core::ImageFormat>, Option<wsi_core::ColorSpace>)],
    mut surface_formats: S,
) -> Option<(vk::Format, vk::ColorSpaceKHR)>
where
    S: FnMut() -> T,
    T: DoubleEndedIterator + Iterator<Item = (vk::Format, vk::ColorSpaceKHR)>,
{
    let surface_formats_set = Thunk::defer(|| HashSet::<_>::from_iter(surface_formats()));
    let surface_formats_by_format = Thunk::defer(|| {
        HashMap::<_, _>::from_iter(surface_formats().rev().filter_map(|x| {
            reverse_translate_color_space(x.1).map(|_| (x.0, x.1))
        }))
    });
    let surface_formats_by_color_space = Thunk::defer(|| {
        HashMap::<_, _>::from_iter(surface_formats().rev().filter_map(|x| {
            reverse_translate_image_format(x.0).map(|_| (x.1, x.0))
        }))
    });

    candidates
        .iter()
        .filter_map(|&(format, color_space)| match (format, color_space) {
            (Some(format), Some(color_space)) => {
                let vk_format = translate_image_format(format);
                let vk_color_space = translate_color_space(color_space);
                if let (Some(vk_format), Some(vk_color_space)) = (vk_format, vk_color_space) {
                    if surface_formats_set.contains(&(vk_format, vk_color_space)) {
                        Some((vk_format, vk_color_space))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            (Some(format), None) => {
                translate_image_format(format).and_then(|vk_format| {
                    surface_formats_by_format.get(&vk_format).map(
                        |&vk_color_space| (vk_format, vk_color_space),
                    )
                })
            }

            (None, Some(color_space)) => {
                translate_color_space(color_space).and_then(|vk_color_space| {
                    surface_formats_by_color_space.get(&vk_color_space).map(
                        |&vk_format| (vk_format, vk_color_space),
                    )
                })
            }
            (None, None) => {
                surface_formats()
                    .filter_map(|(vk_format, vk_color_space)| {
                        let format = reverse_translate_image_format(vk_format);
                        let color_space = reverse_translate_color_space(vk_color_space);
                        if let (Some(_), Some(_)) = (format, color_space) {
                            Some((vk_format, vk_color_space))
                        } else {
                            None
                        }
                    })
                    .nth(0)
            }
        })
        .nth(0)
}
