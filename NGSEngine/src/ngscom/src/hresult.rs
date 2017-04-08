
#![allow(overflowing_literals)]

pub type HResult = i32;

pub const E_OK: HResult = 0;
pub const E_NOTIMPL: HResult = 0x80004001 as i32;
pub const E_NOINTERFACE: HResult = 0x80004002 as i32;
