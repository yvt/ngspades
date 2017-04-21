//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

use std::result::Result;
use std::convert::Into;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[must_use]
pub struct HResult(i32);

impl HResult {
    pub fn new(code: i32) -> Self {
        HResult(code)
    }

    pub fn into_i32(&self) -> i32 {
        self.0
    }

    pub fn into_result(&self) -> Result<i32, ComError> {
        let code = self.into_i32();
        if code >= 0 {
            Ok(code)
        } else {
            Err(ComError::new(code))
        }
    }

    pub fn is_ok(&self) -> bool {
        self.0 >= 0
    }

    pub fn is_err(&self) -> bool {
        self.0 < 0
    }

    pub fn ok(&self) -> Option<i32> {
        self.into_result().ok()
    }

    pub fn err(&self) -> Option<ComError> {
        self.into_result().err()
    }

    pub fn unwrap(&self) -> i32 {
        self.into_result().unwrap()
    }

    pub fn expect(&self, msg: &str) -> i32 {
        self.into_result().expect(msg)
    }
}

impl Into<i32> for HResult {
    fn into(self) -> i32 {
        self.into_i32()
    }
}

impl Into<Result<i32, ComError>> for HResult {
    fn into(self) -> Result<i32, ComError> {
        self.into_result()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ComError(i32);

impl ComError {
    pub fn new(code: i32) -> Self {
        ComError(code)
    }

    pub fn into_i32(&self) -> i32 {
        self.0
    }

    pub fn into_hresult(&self) -> HResult {
        HResult::new(self.0)
    }
}

impl Into<i32> for ComError {
    fn into(self) -> i32 {
        self.into_i32()
    }
}

impl Into<HResult> for ComError {
    fn into(self) -> HResult {
        self.into_hresult()
    }
}


pub mod hresults {
    use super::HResult;

    pub const E_OK: HResult = HResult(0);
    pub const E_NOTIMPL: HResult = HResult(0x80004001u32 as i32);
    pub const E_NOINTERFACE: HResult = HResult(0x80004002u32 as i32);
}

/**
`HResult` counterpart of the standard library's `try!` macro.

# Usage
```
#[macro_use]
extern crate ngscom;
use ngscom::{HResult, hresults};

fn my_little_function() -> HResult {
    hresults::E_OK
}

fn my_greater_function() -> HResult {
    com_try!(my_little_function());
    hresults::E_OK
}

# fn main() { my_greater_function().unwrap(); }
```
*/
#[macro_export]
macro_rules! com_try {
    ($x:expr) => (
        match $x.into_result() {
            Ok(code) => code,
            Err(err) => { return err.into_hresult(); }
        }
    )
}
