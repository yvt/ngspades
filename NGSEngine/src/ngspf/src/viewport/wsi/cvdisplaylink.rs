//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

extern crate block;

use std::os::raw::{c_long, c_void};
use std::ptr;

type CVReturn = i32;
const kCVReturnSuccess: CVReturn = 0;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct CVError(CVReturn);

fn translate_cv_return(x: CVReturn) -> Result<(), CVError> {
    if x == kCVReturnSuccess {
        Ok(())
    } else {
        Err(CVError(x))
    }
}

type CGDirectDisplayID = u32;

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct CVDisplayLinkRef(*const c_void);

unsafe impl Sync for CVDisplayLinkRef {}
unsafe impl Send for CVDisplayLinkRef {}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CVTimeStamp {
    pub version: u32,
    pub videoTimeScale: i32,
    pub videoTime: i64,
    pub hostTime: u64,
    pub rateScalar: f64,
    pub videoRefreshPeriod: i64,
    pub smpteTime: CVSMPTETime,
    pub flags: u64,
    pub reserved: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CVSMPTETime {
    pub subframes: i16,
    pub subframeDivisor: i16,
    pub counter: u32,
    pub typ: u32,
    pub flags: u32,
    pub hours: i16,
    pub minutes: i16,
    pub seconds: i16,
    pub frames: i16,
}

type CVOptionFlags = u64;

type CVDisplayLinkOutputHandler = block::Block<
    (
        CVDisplayLinkRef,
        *const CVTimeStamp,
        *const CVTimeStamp,
        CVOptionFlags,
        *mut CVOptionFlags,
    ),
    CVReturn,
>;

#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    fn CVDisplayLinkCreateWithCGDisplays(
        displayArray: *const CGDirectDisplayID,
        count: c_long,
        displayLinkOut: *mut CVDisplayLinkRef,
    ) -> CVReturn;

    fn CVDisplayLinkCreateWithActiveCGDisplays(displayLinkOut: *mut CVDisplayLinkRef) -> CVReturn;

    fn CVDisplayLinkRelease(displayLink: CVDisplayLinkRef);

    fn CVDisplayLinkStart(displayLink: CVDisplayLinkRef) -> CVReturn;

    fn CVDisplayLinkSetOutputHandler(
        displayLink: CVDisplayLinkRef,
        callback: &CVDisplayLinkOutputHandler,
    ) -> CVReturn;
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct CVDisplayLink {
    raw: CVDisplayLinkRef,
}

impl Drop for CVDisplayLink {
    fn drop(&mut self) {
        unsafe {
            CVDisplayLinkRelease(self.raw);
        }
    }
}

impl CVDisplayLink {
    pub fn new() -> Result<Self, CVError> {
        let mut raw = CVDisplayLinkRef(ptr::null());
        translate_cv_return(unsafe { CVDisplayLinkCreateWithActiveCGDisplays(&mut raw) })?;
        Ok(Self { raw })
    }

    pub fn from_cg_displays(displays: &[CGDirectDisplayID]) -> Result<Self, CVError> {
        let mut raw = CVDisplayLinkRef(ptr::null());
        translate_cv_return(unsafe {
            CVDisplayLinkCreateWithCGDisplays(displays.as_ptr(), displays.len() as c_long, &mut raw)
        })?;
        Ok(Self { raw })
    }

    pub fn start(&self) -> Result<(), CVError> {
        translate_cv_return(unsafe { CVDisplayLinkStart(self.raw) })
    }

    pub fn set_output_callback<F>(&self, f: F) -> Result<(), CVError>
    where
        F: Fn(&CVTimeStamp, &CVTimeStamp, CVOptionFlags, &mut CVOptionFlags) + Send + 'static,
    {
        let block = block::ConcreteBlock::new(
            move |_, in_now: *const _, in_output_time: *const _, flags_in, flags_out: *mut _| {
                unsafe {
                    f(&*in_now, &*in_output_time, flags_in, &mut *flags_out);
                }
                kCVReturnSuccess
            },
        );
        translate_cv_return(unsafe { CVDisplayLinkSetOutputHandler(self.raw, &block.copy()) })
    }
}
