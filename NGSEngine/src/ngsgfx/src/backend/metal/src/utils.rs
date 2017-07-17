//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Deref;
use metal::{self, NSObjectProtocol, id};
use core;

/// Smart pointer for Objective-C objects.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct OCPtr<T: NSObjectProtocol> {
    data: T,
}

impl<T> OCPtr<id<T>>
where
    id<T>: NSObjectProtocol,
{
    pub fn new(ptr: id<T>) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            unsafe {
                ptr.retain();
            }
            Some(Self { data: ptr })
        }
    }

    #[allow(dead_code)]
    pub unsafe fn from_raw(ptr: id<T>) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self { data: ptr })
        }
    }
}

impl<T: NSObjectProtocol> OCPtr<T> {
    #[allow(dead_code)]
    pub fn into_raw(mut this: Self) -> T {
        let ret = unsafe { ::std::mem::replace(&mut this.data, ::std::mem::uninitialized()) };
        ::std::mem::forget(this);
        ret
    }
}

impl<T: NSObjectProtocol> Deref for OCPtr<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: NSObjectProtocol + Clone> Clone for OCPtr<T> {
    fn clone(&self) -> Self {
        let new_data = self.data.clone();
        unsafe { new_data.retain() };
        Self { data: new_data }
    }
}

impl<T: NSObjectProtocol> Drop for OCPtr<T> {
    fn drop(&mut self) {
        unsafe { self.data.release() };
    }
}

pub fn translate_compare_function(value: core::CompareFunction) -> metal::MTLCompareFunction {
    match value {
        core::CompareFunction::NotEqual => metal::MTLCompareFunction::NotEqual,
        core::CompareFunction::Equal => metal::MTLCompareFunction::Equal,
        core::CompareFunction::GreaterEqual => metal::MTLCompareFunction::GreaterEqual,
        core::CompareFunction::Greater => metal::MTLCompareFunction::Greater,
        core::CompareFunction::LessEqual => metal::MTLCompareFunction::LessEqual,
        core::CompareFunction::Less => metal::MTLCompareFunction::Less,
        core::CompareFunction::Never => metal::MTLCompareFunction::Never,
        core::CompareFunction::Always => metal::MTLCompareFunction::Always,
    }
}

pub fn translate_viewport(value: &core::Viewport) -> metal::MTLViewport {
    metal::MTLViewport {
        originX: value.x as f64,
        originY: value.y as f64,
        width: value.width as f64,
        height: value.height as f64,
        znear: value.min_depth as f64,
        zfar: value.max_depth as f64,
    }
}

pub fn translate_scissor_rect(value: &core::Rect2D<u32>) -> metal::MTLScissorRect {
    metal::MTLScissorRect {
        x: value.min.x as u64,
        y: value.min.y as u64,
        width: value.max.x.saturating_sub(value.min.x) as u64,
        height: value.max.y.saturating_sub(value.min.y) as u64,
    }
}
