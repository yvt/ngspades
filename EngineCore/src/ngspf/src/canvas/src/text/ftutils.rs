//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Utilities for interacting with FreeType.
use ::freetype::{freetype, succeeded};
use lazy_static::lazy_static;
use std::cell::{RefCell, RefMut};
use std::mem::ManuallyDrop;
use std::os::raw::{c_int, c_long, c_uint, c_void};
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
}

#[derive(Debug)]
pub struct Library(freetype::FT_Library);

unsafe impl Send for Library {}

impl Library {
    fn new() -> Result<Self, Error> {
        let mut handle: freetype::FT_Library = crate::null_mut();
        Error::from_raw(unsafe { freetype::FT_Init_FreeType(&mut handle) })?;
        Ok(Library(handle))
    }

    pub fn global() -> impl crate::Deref<Target = Library> + 'static {
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
        let mut handle: freetype::FT_Face = crate::null_mut();

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
                ManuallyDrop::new(unsafe { Face::from_raw(handle) }),
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

pub unsafe trait FaceBuffer: crate::Debug + Sync + Send {
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

impl crate::Deref for MemoryFace {
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
pub struct Face(freetype::FT_Face, RefCell<Outline>);

unsafe impl Send for Face {}

impl Face {
    unsafe fn from_raw(raw: freetype::FT_Face) -> Self {
        let outline = Outline(&mut (*(*raw).glyph).outline);
        Face(raw, RefCell::new(outline))
    }
    pub fn raw(&self) -> freetype::FT_Face {
        self.0
    }

    pub fn set_char_size(
        &self,
        char_width: c_long,
        char_height: c_long,
        horz_res: u32,
        vert_res: u32,
    ) -> Result<(), Error> {
        Error::from_raw(unsafe {
            freetype::FT_Set_Char_Size(self.0, char_width, char_height, horz_res, vert_res)
        })
    }

    pub fn load_glyph(&self, glyph_index: c_uint, load_flags: i32) -> Result<(), Error> {
        Error::from_raw(unsafe { freetype::FT_Load_Glyph(self.0, glyph_index, load_flags) })
    }

    pub fn glyph_slot_outline(&self) -> RefMut<Outline> {
        self.1.borrow_mut()
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        // Assuming this `FT_Face` was allocated from it...
        let _lock = Library::global();
        Error::from_raw(unsafe { freetype::FT_Done_Face(self.0) }).unwrap();
    }
}

#[derive(Debug)]
pub struct Outline(*mut freetype::FT_Outline);

impl Outline {
    pub fn translate(&mut self, x: c_long, y: c_long) {
        unsafe {
            freetype::FT_Outline_Translate(self.0, x, y);
        }
    }

    pub fn transform(&mut self, matrix: &freetype::FT_Matrix) {
        unsafe {
            freetype::FT_Outline_Transform(self.0, matrix);
        }
    }

    pub fn render_direct<F>(
        &mut self,
        library: &Library,
        clip_box: Option<freetype::FT_BBox>,
        mut f: F,
    ) -> Result<(), Error>
    where
        F: FnMut(c_int, &[freetype::FT_Span]),
    {
        unsafe extern "C" fn gray_spans<F>(
            y: c_int,
            count: c_int,
            spans: *const freetype::FT_Span,
            user: *mut c_void,
        ) where
            F: FnMut(c_int, &[freetype::FT_Span]),
        {
            use std::slice::from_raw_parts;
            let ref mut cb = *(user as *mut F);
            cb(y, from_raw_parts(spans, count as _));
        }

        let mut params = freetype::FT_Raster_Params {
            target: crate::null(),
            source: crate::null(),
            flags: (freetype::FT_RASTER_FLAG_DIRECT
                | freetype::FT_RASTER_FLAG_AA
                | if clip_box.is_some() {
                    freetype::FT_RASTER_FLAG_CLIP
                } else {
                    0
                }) as i32,
            gray_spans: Some(gray_spans::<F>),
            black_spans: None,
            bit_test: None,
            bit_set: None,
            user: (&mut f) as *mut _ as *mut c_void,
            clip_box: clip_box.unwrap_or(freetype::FT_BBox {
                xMin: 0,
                yMin: 0,
                xMax: 0,
                yMax: 0,
            }),
        };

        Error::from_raw(unsafe { freetype::FT_Outline_Render(library.raw(), self.0, &mut params) })
    }
}
