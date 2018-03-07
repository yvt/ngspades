//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;

use common::{Error, ErrorKind};

/// Translates a subset of `vk::Result` values into `core::GenericError`.
///
/// The following input values are permitted:
///
///  - `ErrorOutOfDeviceMemory`
///  - `ErrorDeviceLost`
///
/// `ErrorOutOfHostMemory` is escalated to a panic. (Maybe we should call `alloc::oom::oom()`?)
///
/// Unsupported values are returned unmodified.
pub fn translate_generic_error(result: vk::Result) -> ::std::result::Result<Error, vk::Result> {
    match result {
        vk::Result::ErrorOutOfDeviceMemory => Ok(Error::new(ErrorKind::OutOfDeviceMemory)),
        vk::Result::ErrorDeviceLost => Ok(Error::new(ErrorKind::DeviceLost)),
        vk::Result::ErrorOutOfHostMemory => panic!("out of memory"),
        result => Err(result),
    }
}

/// Equivalent to `translate_generic_error(result).unwrap()`.
///
/// That is, following errors are handled with this function:
///
///  - `ErrorOutOfDeviceMemory`
///  - `ErrorDeviceLost`
///  - `ErrorOutOfHostMemory` (escalated to a panic)
///
pub fn translate_generic_error_unwrap(result: vk::Result) -> Error {
    translate_generic_error(result).unwrap()
}

pub(crate) fn translate_map_memory_error(
    result: vk::Result,
) -> ::std::result::Result<Error, vk::Result> {
    match result {
        vk::Result::ErrorMemoryMapFailed => panic!("out of virtual memory space"),
        result => translate_generic_error(result),
    }
}

pub(crate) fn translate_map_memory_error_unwrap(result: vk::Result) -> Error {
    translate_map_memory_error(result).unwrap()
}
