//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! The Nightingales engine loader helper.
//!
//! This dynamic library provides a helper function to examine the processor's
//! capability. The actual loader (probably implemented in C♯) utilizes it to
//! dispatch the game engine core binary ([`ngsengine`]) that run most
//! efficiently on the user's machine.
//!
//! [`ngsengine`]: ../ngsengine/index.html
//!
#![feature(target_feature)]
// Use the system allocator - we don't want two instances of jemalloc running
// simultaneously! Besides, we don't care about the allocator's performance here.
#![feature(alloc_system)]
extern crate alloc_system;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate ngscom;
extern crate ngsbase;

use ngsbase::IProcessorInfo;
use ngscom::{hresults, ComPtr, HResult};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86::ProcessorInfo;

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
mod generic;
#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
use generic::ProcessorInfo;

#[no_mangle]
pub unsafe extern "C" fn ngsloader_get_processor_info(
    retval: &mut ComPtr<IProcessorInfo>,
) -> HResult {
    *retval = ProcessorInfo::new();
    hresults::E_OK
}