/*!
# ngscom
Nightingales COM (NGSCOM) for Rust
*/

//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//
// This source code is based on com-rs. The original license text is shown below:
//
//     Copyright (c) 2016 com-rs developers
//
//     Licensed under the Apache License, Version 2.0
//     <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
//     license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
//     option. All files in the project carrying such notice may not be copied,
//     modified, or distributed except according to those terms.
//

// #![deny(dead_code)]
#![deny(missing_debug_implementations)]
// #![deny(missing_docs)]

mod bstring;
#[macro_use] mod implmacros;

pub use bstring::{BString, BStringVtable, BStringRef};

/*
# com-rs 0.1.4
Rust bindings for the Win32 [Component Object Model]
(https://msdn.microsoft.com/en-us/library/ms680573.aspx).

# Overview
This crate is composed of three main components:

* The [`com_interface!`] (macro.com_interface!.html) macro for
  defining new interface types.
* The [`ComPtr`](struct.ComPtr.html) type for making use of them.
* Definition of [`IUnknown`](struct.IUnknown.html), the base COM interface.
*/

// TODO:
// * Implement the rest of COM, this is just a tiny subset necessary to consume
//   IUnknown interfaces.
// * Tests for IUnknown/ComPtr, hard to test with no way of acquiring
//   IUnknown objects directly.


use std::fmt;

pub use comptr::{AsComPtr, ComInterface, ComPtr};
pub use iunknown::{IUnknown, IUnknownTrait};

/// Interface identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct IID {
    /// First component, 32-bit value.
    pub data1: u32,
    /// Second component, 16-bit value.
    pub data2: u16,
    /// Third component, 16-bit value.
    pub data3: u16,
    /// Fourth component, array of 8-bit values.
    pub data4: [u8; 8],
}

pub trait StaticOffset {
    fn offset() -> isize;
}

#[allow(missing_debug_implementations)]
pub struct StaticZeroOffset();

impl StaticOffset for StaticZeroOffset {
    fn offset() -> isize { 0 }
}

/// Print IID in Windows registry format {XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}.
impl fmt::Display for IID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-\
                   {:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
            self.data1, self.data2, self.data3,
            self.data4[0], self.data4[1], self.data4[2], self.data4[3],
            self.data4[4], self.data4[5], self.data4[6], self.data4[7])
    }
}

#[test]
fn iid_display() {
    assert_eq!(IUnknown::iid().to_string(),
               "{00000000-0000-0000-C000-000000000046}");
}

/// Result type.
pub type HResult = i32;

pub const E_OK: HResult = 0;
#[allow(overflowing_literals)]
pub const E_NOINTERFACE: HResult = 0x80004002 as i32;

#[macro_use] mod ifacemacros;

mod comptr;
mod iunknown;

// Utility functions for macros
#[doc(hidden)]
pub mod detail;
