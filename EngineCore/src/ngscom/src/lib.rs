//! Nightingales COM (NgsCOM) for Rust
//! ==================================
//!
//! Credits
//! -------
//!
//! ### com-rs
//!
//! A large portion of this crate is based on [com-rs](https://github.com/Eljay/com-rs),
//! the Rust bindings for the Win32 Component Object Model, licensed under the MIT license.
//!
//! Copyright 2016 com-rs developers
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy of this
//! software and associated documentation files (the "Software"), to deal in the Software
//! without restriction, including without limitation the rights to use, copy, modify,
//! merge, publish, distribute, sublicense, and/or sell copies of the Software, and to
//! permit persons to whom the Software is furnished to do so, subject to the following
//! conditions:
//!
//! The above copyright notice and this permission notice shall be included in all copies
//! or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
//! INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
//! PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT
//! HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
//! CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
//! OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//!
//!
//! License
//! -------
//!
//! Follows the license of the parent project (Nightingales).

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
#![warn(rust_2018_idioms)]
// #![deny(dead_code)]
#![deny(missing_debug_implementations)]
// #![deny(missing_docs)]

/// This re-export is accessed by clients via `com_vtable!`.
#[doc(hidden)]
pub use lazy_static::lazy_static;

mod bstring;
#[macro_use]
mod implmacros;

pub use crate::bstring::{BString, BStringRef, BStringVtable};

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

pub use crate::comptr::{AsComPtr, ComInterface, ComPtr};
pub use crate::iany::{IAny, IAnyTrait, IAnyVTable};
pub use crate::iunknown::{IUnknown, IUnknownTrait};
pub use crate::unownedcomptr::UnownedComPtr;

/// An interface identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct IID {
    /// The first component, 32-bit value.
    pub data1: u32,
    /// The second component, 16-bit value.
    pub data2: u16,
    /// The third component, 16-bit value.
    pub data3: u16,
    /// The fourth component, array of 8-bit values.
    pub data4: [u8; 8],
}

pub trait StaticOffset {
    fn offset() -> isize;
}

#[allow(missing_debug_implementations)]
pub struct StaticZeroOffset();

impl StaticOffset for StaticZeroOffset {
    fn offset() -> isize {
        0
    }
}

/// Prints IID in Windows registry format {XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX}.
impl fmt::Display for IID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-\
             {:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
            self.data1,
            self.data2,
            self.data3,
            self.data4[0],
            self.data4[1],
            self.data4[2],
            self.data4[3],
            self.data4[4],
            self.data4[5],
            self.data4[6],
            self.data4[7]
        )
    }
}

#[test]
fn iid_display() {
    assert_eq!(
        IUnknown::iid().to_string(),
        "{00000000-0000-0000-C000-000000000046}"
    );
}

#[macro_use]
mod hresult;
pub use crate::hresult::*;
#[macro_use]
mod ifacemacros;
mod comptr;
mod iany;
mod iunknown;
mod unownedcomptr;
mod utils;
pub use crate::utils::*;

// Utility functions for macros
#[doc(hidden)]
pub mod detail;
