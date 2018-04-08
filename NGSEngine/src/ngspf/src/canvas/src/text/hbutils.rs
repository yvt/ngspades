//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Utilities for interacting with HarfBuzz.
use harfbuzz;

#[derive(Debug)]
pub struct Blob(*mut harfbuzz::hb_blob_t);

unsafe impl Sync for Blob {}
unsafe impl Send for Blob {}

impl Blob {
    unsafe fn from_raw(raw: *mut harfbuzz::hb_blob_t) -> Self {
        Blob(raw)
    }

    pub unsafe fn new_pinned(data: &[u8]) -> Self {
        use std::mem::transmute;

        let raw = harfbuzz::RUST_hb_blob_create(
            data.as_ptr() as *const _,
            data.len() as _,
            harfbuzz::HB_MEMORY_MODE_READONLY,
            ::null_mut(),
            transmute(::null::<u8>()),
        );
        assert_ne!(raw, ::null_mut());
        Self::from_raw(raw)
    }

    pub fn get(&self) -> *mut harfbuzz::hb_blob_t {
        self.0
    }
}

impl Drop for Blob {
    fn drop(&mut self) {
        unsafe {
            harfbuzz::RUST_hb_blob_destroy(self.0);
        }
    }
}

impl Clone for Blob {
    fn clone(&self) -> Self {
        unsafe { Blob(harfbuzz::RUST_hb_blob_reference(self.0)) }
    }
}

#[derive(Debug)]
pub struct Face(*mut harfbuzz::hb_face_t);

unsafe impl Sync for Face {}
unsafe impl Send for Face {}

impl Face {
    unsafe fn from_raw(raw: *mut harfbuzz::hb_face_t) -> Self {
        Face(raw)
    }

    pub fn new(blob: &Blob, index: u32) -> Self {
        let raw = unsafe { harfbuzz::RUST_hb_face_create(blob.get(), index) };
        assert_ne!(raw, ::null_mut());
        unsafe { Self::from_raw(raw) }
    }

    pub fn get(&self) -> *mut harfbuzz::hb_face_t {
        self.0
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        unsafe {
            harfbuzz::RUST_hb_face_destroy(self.0);
        }
    }
}

impl Clone for Face {
    fn clone(&self) -> Self {
        unsafe { Face(harfbuzz::RUST_hb_face_reference(self.0)) }
    }
}

#[derive(Debug)]
pub struct Font(*mut harfbuzz::hb_font_t);

unsafe impl Sync for Font {}
unsafe impl Send for Font {}

impl Font {
    unsafe fn from_raw(raw: *mut harfbuzz::hb_font_t) -> Self {
        Font(raw)
    }

    pub fn new(face: &Face) -> Self {
        let raw = unsafe { harfbuzz::RUST_hb_font_create(face.get()) };
        assert_ne!(raw, ::null_mut());
        unsafe { Self::from_raw(raw) }
    }

    pub fn get(&self) -> *mut harfbuzz::hb_font_t {
        self.0
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            harfbuzz::RUST_hb_font_destroy(self.0);
        }
    }
}

impl Clone for Font {
    fn clone(&self) -> Self {
        unsafe { Font(harfbuzz::RUST_hb_font_reference(self.0)) }
    }
}
