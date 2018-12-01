//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Utilities for interacting with HarfBuzz.
use std::slice;

#[derive(Debug)]
pub struct Blob(*mut harfbuzz_sys::hb_blob_t);

unsafe impl Sync for Blob {}
unsafe impl Send for Blob {}

impl Blob {
    unsafe fn from_raw(raw: *mut harfbuzz_sys::hb_blob_t) -> Self {
        Blob(raw)
    }

    pub unsafe fn new_pinned(data: &[u8]) -> Self {
        use std::mem::transmute;

        let raw = harfbuzz_sys::hb_blob_create(
            data.as_ptr() as *const _,
            data.len() as _,
            harfbuzz_sys::HB_MEMORY_MODE_READONLY,
            crate::null_mut(),
            transmute(crate::null::<u8>()),
        );
        assert_ne!(raw, crate::null_mut());
        Self::from_raw(raw)
    }

    pub fn get(&self) -> *mut harfbuzz_sys::hb_blob_t {
        self.0
    }
}

impl Drop for Blob {
    fn drop(&mut self) {
        unsafe {
            harfbuzz_sys::hb_blob_destroy(self.0);
        }
    }
}

impl Clone for Blob {
    fn clone(&self) -> Self {
        unsafe { Blob(harfbuzz_sys::hb_blob_reference(self.0)) }
    }
}

#[derive(Debug)]
pub struct Face(*mut harfbuzz_sys::hb_face_t);

unsafe impl Sync for Face {}
unsafe impl Send for Face {}

impl Face {
    unsafe fn from_raw(raw: *mut harfbuzz_sys::hb_face_t) -> Self {
        Face(raw)
    }

    pub fn new(blob: &Blob, index: u32) -> Self {
        let raw = unsafe { harfbuzz_sys::hb_face_create(blob.get(), index) };
        assert_ne!(raw, crate::null_mut());
        unsafe { Self::from_raw(raw) }
    }

    pub fn get(&self) -> *mut harfbuzz_sys::hb_face_t {
        self.0
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        unsafe {
            harfbuzz_sys::hb_face_destroy(self.0);
        }
    }
}

impl Clone for Face {
    fn clone(&self) -> Self {
        unsafe { Face(harfbuzz_sys::hb_face_reference(self.0)) }
    }
}

#[derive(Debug)]
pub struct Font(*mut harfbuzz_sys::hb_font_t);

unsafe impl Sync for Font {}
unsafe impl Send for Font {}

impl Font {
    unsafe fn from_raw(raw: *mut harfbuzz_sys::hb_font_t) -> Self {
        Font(raw)
    }

    pub fn new(face: &Face) -> Self {
        let raw = unsafe { harfbuzz_sys::hb_font_create(face.get()) };
        assert_ne!(raw, crate::null_mut());
        unsafe {
            harfbuzz_sys::hb_ot_font_set_funcs(raw);
        }
        unsafe { Self::from_raw(raw) }
    }

    pub fn get(&self) -> *mut harfbuzz_sys::hb_font_t {
        self.0
    }

    pub fn shape(&self, buffer: &mut Buffer) {
        unsafe {
            harfbuzz_sys::hb_shape(self.0, buffer.0, crate::null_mut(), 0);
        }
    }

    pub fn set_scale(&mut self, x: i32, y: i32) {
        unsafe {
            harfbuzz_sys::hb_font_set_scale(self.0, x, y);
        }
    }

    pub fn glyph_extents(&self, glyph: u32) -> Option<harfbuzz_sys::hb_glyph_extents_t> {
        let mut extents = harfbuzz_sys::hb_glyph_extents_t {
            x_bearing: 0,
            y_bearing: 0,
            width: 0,
            height: 0,
        };
        if unsafe { harfbuzz_sys::hb_font_get_glyph_extents(self.0, glyph, &mut extents) } != 0 {
            Some(extents)
        } else {
            None
        }
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            harfbuzz_sys::hb_font_destroy(self.0);
        }
    }
}

impl Clone for Font {
    fn clone(&self) -> Self {
        unsafe { Font(harfbuzz_sys::hb_font_reference(self.0)) }
    }
}

#[derive(Debug)]
pub struct Buffer(*mut harfbuzz_sys::hb_buffer_t);

unsafe impl Sync for Buffer {}
unsafe impl Send for Buffer {}

impl Buffer {
    unsafe fn from_raw(raw: *mut harfbuzz_sys::hb_buffer_t) -> Self {
        Buffer(raw)
    }

    pub fn new() -> Self {
        let raw = unsafe { harfbuzz_sys::hb_buffer_create() };
        assert_ne!(raw, crate::null_mut());
        unsafe { Self::from_raw(raw) }
    }

    pub fn set_direction(&mut self, x: harfbuzz_sys::hb_direction_t) {
        unsafe {
            harfbuzz_sys::hb_buffer_set_direction(self.0, x);
        }
    }

    pub fn set_script(&mut self, x: Script) {
        unsafe {
            harfbuzz_sys::hb_buffer_set_script(self.0, x.0);
        }
    }

    pub fn set_language(&mut self, x: Language) {
        unsafe {
            harfbuzz_sys::hb_buffer_set_language(self.0, x.0);
        }
    }

    pub fn set_content_type(&mut self, x: harfbuzz_sys::hb_buffer_content_type_t) {
        unsafe {
            harfbuzz_sys::hb_buffer_set_content_type(self.0, x);
        }
    }

    pub fn add(&mut self, codepoint: char, cluster: u32) {
        unsafe {
            harfbuzz_sys::hb_buffer_add(self.0, codepoint as u32, cluster);
        }
    }

    pub fn guess_segment_properties(&mut self) {
        unsafe {
            harfbuzz_sys::hb_buffer_guess_segment_properties(self.0);
        }
    }

    pub fn glyph_infos(&self) -> &[harfbuzz_sys::hb_glyph_info_t] {
        let mut len = 0;
        let ptr = unsafe { harfbuzz_sys::hb_buffer_get_glyph_infos(self.0, &mut len) };
        assert!(!ptr.is_null() || len == 0);
        unsafe { slice::from_raw_parts(ptr, len as usize) }
    }

    pub fn glyph_positions(&self) -> &[harfbuzz_sys::hb_glyph_position_t] {
        let mut len = 0;
        let ptr = unsafe { harfbuzz_sys::hb_buffer_get_glyph_positions(self.0, &mut len) };
        assert!(!ptr.is_null() || len == 0);
        unsafe { slice::from_raw_parts(ptr, len as usize) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            harfbuzz_sys::hb_buffer_destroy(self.0);
        }
    }
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        unsafe { Buffer(harfbuzz_sys::hb_buffer_reference(self.0)) }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct Language(harfbuzz_sys::hb_language_t);

unsafe impl Sync for Language {}
unsafe impl Send for Language {}

impl Language {
    pub unsafe fn from_raw(x: harfbuzz_sys::hb_language_t) -> Self {
        Language(x)
    }

    pub fn from_iso639(x: &str) -> Option<Self> {
        let x = unsafe {
            harfbuzz_sys::hb_language_from_string(x.as_ptr() as *mut u8 as *mut _, x.len() as _)
        };
        use std::ptr::null_mut;
        if x == null_mut() {
            None
        } else {
            Some(Language(x))
        }
    }
}

impl Default for Language {
    fn default() -> Self {
        Language(unsafe { harfbuzz_sys::hb_language_get_default() })
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct Script(harfbuzz_sys::hb_script_t);

impl Script {
    pub unsafe fn from_raw(x: harfbuzz_sys::hb_script_t) -> Self {
        Script(x)
    }

    pub fn from_iso15924(x: &str) -> Option<Self> {
        let x = unsafe {
            harfbuzz_sys::hb_script_from_string(x.as_ptr() as *mut u8 as *mut _, x.len() as _)
        };
        if x == harfbuzz_sys::HB_SCRIPT_INVALID {
            None
        } else {
            Some(Script(x))
        }
    }
}

impl Default for Script {
    /// Return `Script::from_raw(harfbuzz_sys::HB_SCRIPT_COMMON)`.
    fn default() -> Self {
        Script(harfbuzz_sys::HB_SCRIPT_COMMON)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_values() {
        Language::default();
        Script::default();
    }
}
