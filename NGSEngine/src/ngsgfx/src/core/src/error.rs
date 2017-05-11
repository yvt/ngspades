//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/// Defines generic error codes.
/// Some common errors are not included for the following reasons:
///
/// - **Invalid usage**: the backend assumes valid usage, and in case where an invalid
///   usage is detected, it would `panic`.
/// - **Out of host memory**: as per common conventions of Rust, out of memory would result in
///   abort. (`panic` is also permitted since `abort` is a nightly-only API)
/// - **Not supported**: the application must check parameters beforehand so it doesn't use any
///   features unsupported by the backend or the hardware.
///   This counts as an invalid usage. TODO: how to check?
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum GenericError {
    OutOfDeviceMemory,
}

pub type Result<T> = ::std::result::Result<T, GenericError>;
