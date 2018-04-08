//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Utilities for interacting with FreeType.
use freetype::{freetype, succeeded};
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Error(freetype::FT_Error);

impl Error {
    pub fn from_raw(x: freetype::FT_Error) -> Result<(), Self> {
        if succeeded(x) {
            Ok(())
        } else {
            Err(Error(x))
        }
    }

    pub fn raw(&self) -> freetype::FT_Error {
        self.0
    }
}

#[derive(Debug)]
pub struct Library(freetype::FT_Library);

unsafe impl Send for Library {}

impl Library {
    fn new() -> Result<Self, Error> {
        let mut handle: freetype::FT_Library = ::null_mut();
        Error::from_raw(unsafe { freetype::FT_Init_FreeType(&mut handle) })?;
        Ok(Library(handle))
    }

    pub fn global() -> impl ::Deref<Target = Library> + 'static {
        lazy_static! {
            static ref LIBRARY: Mutex<Library> = Mutex::new(Library::new().unwrap());
        }
        LIBRARY.lock().unwrap()
    }

    pub fn raw(&self) -> freetype::FT_Library {
        self.0
    }

    pub fn new_memory_face<T: FaceBuffer + 'static>(
        &self,
        buffer: T,
        face_index: i32,
    ) -> Result<MemoryFace, (Error, T)> {
        let mut handle: freetype::FT_Face = ::null_mut();

        match Error::from_raw(unsafe {
            freetype::FT_New_Memory_Face(
                self.0,
                buffer.as_slice().as_ptr(),
                buffer.as_slice().len() as _,
                face_index as _,
                &mut handle,
            )
        }) {
            Ok(()) => Ok(MemoryFace(
                ManuallyDrop::new(Face(handle)),
                Box::new(buffer),
            )),
            Err(x) => Err((x, buffer)),
        }
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        Error::from_raw(unsafe { freetype::FT_Done_FreeType(self.0) }).unwrap();
    }
}

pub unsafe trait FaceBuffer: ::Debug {
    /// Return the contents as a slice. The returned slice must **not** move
    /// throughout the lifetime of `self` (which is why this trait is marked as
    /// `unsafe`).
    fn as_slice(&self) -> &[u8];
}

unsafe impl FaceBuffer for [u8] {
    fn as_slice(&self) -> &[u8] {
        self
    }
}

// `&` does not provide an interior mutability, so it's safe to implement it
unsafe impl<'a, T: FaceBuffer + ?Sized> FaceBuffer for &'a T {
    fn as_slice(&self) -> &[u8] {
        (*self).as_slice()
    }
}

// `Vec` does not provide an interior mutability, so it's safe to implement it
unsafe impl FaceBuffer for Vec<u8> {
    fn as_slice(&self) -> &[u8] {
        &self[..]
    }
}

// `Arc` does not provide an interior mutability, so it's safe to implement it
unsafe impl<T: FaceBuffer> FaceBuffer for Arc<T> {
    fn as_slice(&self) -> &[u8] {
        (**self).as_slice()
    }
}

#[derive(Debug)]
pub struct MemoryFace(ManuallyDrop<Face>, Box<FaceBuffer>);

impl ::Deref for MemoryFace {
    type Target = Face;

    fn deref(&self) -> &Face {
        &self.0
    }
}

impl Drop for MemoryFace {
    fn drop(&mut self) {
        // Drop `Face` first
        unsafe {
            ManuallyDrop::drop(&mut self.0);
        }
    }
}

#[derive(Debug)]
pub struct Face(freetype::FT_Face);

unsafe impl Send for Face {}

impl Face {
    pub fn raw(&self) -> freetype::FT_Face {
        self.0
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        // Assuming this `FT_Face` was allocated from it...
        let _lock = Library::global();
        Error::from_raw(unsafe { freetype::FT_Done_Face(self.0) }).unwrap();
    }
}
