//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Deref;
use std::fmt;
use metal::{self, id, MTLDevice, NSObjectProtocol};
use base;
use common;

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

pub fn translate_cmp_fn(value: base::CmpFn) -> metal::MTLCompareFunction {
    match value {
        base::CmpFn::NotEqual => metal::MTLCompareFunction::NotEqual,
        base::CmpFn::Equal => metal::MTLCompareFunction::Equal,
        base::CmpFn::GreaterEqual => metal::MTLCompareFunction::GreaterEqual,
        base::CmpFn::Greater => metal::MTLCompareFunction::Greater,
        base::CmpFn::LessEqual => metal::MTLCompareFunction::LessEqual,
        base::CmpFn::Less => metal::MTLCompareFunction::Less,
        base::CmpFn::Never => metal::MTLCompareFunction::Never,
        base::CmpFn::Always => metal::MTLCompareFunction::Always,
    }
}

pub fn translate_storage_mode(
    value: base::MemoryType,
) -> Result<metal::MTLStorageMode, base::MemoryType> {
    if value == ::MEMORY_TYPE_PRIVATE {
        Ok(metal::MTLStorageMode::Private)
    } else if value == ::MEMORY_TYPE_SHARED {
        Ok(metal::MTLStorageMode::Shared)
    } else {
        Err(value)
    }
}

pub fn translate_render_stage(stage: base::StageFlags) -> metal::MTLRenderStages {
    let mut stages = metal::MTLRenderStages::empty();

    if stage.intersects(flags![
        base::Stage::{IndirectDraw | VertexInput | Vertex}])
    {
        stages |= metal::MTLRenderStageVertex;
    }

    if stage.intersects(flags![
        base::Stage::{Fragment | EarlyFragTests | LateFragTests | RenderOutput}])
    {
        stages |= metal::MTLRenderStageFragment;
    }

    stages
}

pub fn translate_viewport(value: &base::Viewport) -> metal::MTLViewport {
    metal::MTLViewport {
        originX: value.x as f64,
        originY: value.y as f64,
        width: value.width as f64,
        height: value.height as f64,
        znear: value.min_depth as f64,
        zfar: value.max_depth as f64,
    }
}

pub fn translate_scissor_rect(value: &base::Rect2D<u32>) -> metal::MTLScissorRect {
    metal::MTLScissorRect {
        x: value.min[0] as u64,
        y: value.min[1] as u64,
        width: value.max[0].saturating_sub(value.min[0]) as u64,
        height: value.max[1].saturating_sub(value.min[1]) as u64,
    }
}

pub fn clip_scissor_rect(
    value: &metal::MTLScissorRect,
    extents: &[u32; 2],
) -> metal::MTLScissorRect {
    let (mut x1, mut x2) = (value.x, value.x + value.width);
    let (mut y1, mut y2) = (value.y, value.y + value.height);
    if x1 > extents[0] as u64 {
        x1 = extents[0] as u64;
    }
    if x2 > extents[0] as u64 {
        x2 = extents[0] as u64;
    }
    if y1 > extents[1] as u64 {
        y1 = extents[1] as u64;
    }
    if y2 > extents[1] as u64 {
        y2 = extents[1] as u64;
    }
    metal::MTLScissorRect {
        x: x1,
        y: y1,
        width: x2 - x1,
        height: y2 - y1,
    }
}

#[derive(Debug, Clone, Copy)]
struct SelectorReturnedNullError {
    sel: &'static str,
}

impl ::std::error::Error for SelectorReturnedNullError {
    fn description(&self) -> &str {
        "selector returned nil"
    }
}

impl fmt::Display for SelectorReturnedNullError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Selector '{}' returned nil", self.sel)
    }
}

/// Construct a `common::Error` indicating a selector returned the `nil` value.
pub fn nil_error(sel: &'static str) -> common::Error {
    common::Error::with_detail(common::ErrorKind::Other, SelectorReturnedNullError { sel })
}

pub fn get_memory_req(
    metal_device: MTLDevice,
    obj: base::ResourceRef,
) -> common::Result<base::MemoryReq> {
    use {buffer, image};
    match obj {
        base::ResourceRef::Buffer(buffer) => {
            let our_buffer: &buffer::Buffer = buffer.downcast_ref().expect("bad buffer type");
            Ok(our_buffer.memory_req(metal_device))
        }
        base::ResourceRef::Image(image) => {
            let our_image: &image::Image = image.downcast_ref().expect("bad image type");
            Ok(our_image.memory_req(metal_device))
        }
    }
}