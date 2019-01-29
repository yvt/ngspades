//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{sync::Arc, mem::transmute};
use tokenlock::{Token, TokenRef};

use crate::{AsRawPtr, PtrSized, RcLike};

unsafe impl PtrSized for TokenRef {
    type Value = usize;

    fn into_raw(this: Self) -> *const Self::Value {
        Arc::into_raw(unsafe { transmute::<_, Arc<Self::Value>>(this) })
    }
    unsafe fn from_raw(ptr: *const Self::Value) -> Self {
        transmute::<Arc<Self::Value>, _>(Arc::from_raw(ptr))
    }
}
unsafe impl RcLike for TokenRef {}

impl AsRawPtr<usize> for Token {
    fn as_raw_ptr(&self) -> *const usize {
        unsafe { transmute::<_, &Arc<usize>>(self) }.as_raw_ptr()
    }
}
