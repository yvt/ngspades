//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use self::metal::{id, NSObjectProtocol};
use std::ops::Deref;
use zangfx::backends::metal::metal;

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
