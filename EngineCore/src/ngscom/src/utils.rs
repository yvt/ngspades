//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use hresults::E_OK;
use HResult;

/// Call a given closure and convert its return value of type
/// `Result<(), HResult>` to `HResult`.
pub fn to_hresult<F: FnOnce() -> Result<(), HResult>>(f: F) -> HResult {
    match f() {
        Ok(()) => E_OK,
        Err(e) => e,
    }
}
