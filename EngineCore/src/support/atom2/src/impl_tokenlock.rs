//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{sync::Arc, mem::transmute, ptr::NonNull};
use tokenlock::{Token, TokenRef};

use crate::{AsRawPtr, PtrSized, TypedPtrSized};

#[derive(Debug, Copy, Clone,)]
pub struct TokenValue(usize);

// This implementation is highly dependent on the internals of `tokenlock`,
// unfortunately.
unsafe impl PtrSized for TokenRef {
    fn into_raw(this: Self) -> NonNull<()> {
        let p = Arc::into_raw(unsafe { transmute::<_, Arc<usize>>(this) });
        NonNull::new(p as *mut ()).expect("pointer is unexpectedly null")
    }
    unsafe fn from_raw(ptr: NonNull<()>) -> Self {
        transmute::<Arc<usize>, _>(Arc::from_raw(ptr.as_ptr() as _))
    }
}

unsafe impl TypedPtrSized for TokenRef {
    type Target = usize;
}

impl AsRawPtr<usize> for Token {
    fn as_raw_ptr(&self) -> *const usize {
        unsafe { transmute::<_, &Arc<usize>>(self) }.as_raw_ptr()
    }
}
