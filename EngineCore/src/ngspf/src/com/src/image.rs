//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use arclock::{ArcLock, ArcLockGuard};
use cgmath::Vector2;
use ngscom::{hresults, to_hresult, ComPtr, HResult, IAny, IUnknown};
use owning_ref::OwningRefMut;
use std::sync::Mutex;

use canvas::{ImageData, ImageFormat, ImageRef};
use hresults::E_PF_THREAD;
use ngsbase::{self, IBitmap, IBitmapTrait, IPainter};

// The methods provided by `IBitmap` are inherently unsafe. This unsafeness is
// hidden from partially-trusted assemblies using the .NET wrapper class `Bitmap`.

com_impl! {
    /// A COM wrapper for `ngspf::canvas::ImageData`.
    class ComBitmap {
        ibitmap: IBitmap;
        iany: IAny;
        @data: BitmapData;
    }
}

pub type ComBitmapLockGuard = OwningRefMut<ArcLockGuard<Option<ImageData>>, ImageData>;

#[derive(Debug)]
struct BitmapData {
    image_data: ArcLock<Option<ImageData>>,
    contents_ptr: usize,
    format: ngsbase::PixelFormat,
    size: Vector2<i32>,

    /// Stores a lock guard obtained by `IBitmapTrait::lock`.
    guard_cell: Mutex<Option<ArcLockGuard<Option<ImageData>>>>,
}

impl ComBitmap {
    /// Construct a `ComBitmap` and get it as an `IUnknown`.
    pub fn new(mut image_data: ImageData) -> ComPtr<IUnknown> {
        let contents_ptr = image_data.pixels_u32_mut().as_ptr() as usize;
        let format = match image_data.format() {
            ImageFormat::SrgbRgba8 => ngsbase::PixelFormatItem::SrgbRgba8,
            ImageFormat::SrgbRgba8Premul => ngsbase::PixelFormatItem::SrgbRgba8Premul,
        }.into();
        let size = image_data.size().cast::<i32>();

        Self::alloc(BitmapData {
            image_data: ArcLock::new(Some(image_data)),
            contents_ptr,
            format,
            size,
            guard_cell: Mutex::new(None),
        })
    }

    /// Acquire a lock on the contained `ImageData`. Returns `None` if it's been
    /// already converted into an immutable image.
    pub fn lock_image_data(&self) -> Option<ComBitmapLockGuard> {
        let ref data: BitmapData = self.data;
        OwningRefMut::new(data.image_data.lock().unwrap())
            .try_map_mut(|guard| guard.as_mut().ok_or(()))
            .ok()
    }
}

impl IBitmapTrait for ComBitmap {
    fn clone(&self, retval: &mut ComPtr<IBitmap>) -> HResult {
        to_hresult(|| {
            let image_data = self.lock_image_data().ok_or(hresults::E_UNEXPECTED)?;
            let new_image_data = image_data.clone();
            *retval = (&ComBitmap::new(new_image_data)).into();
            Ok(())
        })
    }

    fn create_painter(&self, _retval: &mut ComPtr<IPainter>) -> HResult {
        to_hresult(|| {
            self.lock_image_data().ok_or(hresults::E_UNEXPECTED)?;
            // TODO: Create painter
            Err(hresults::E_NOTIMPL)
        })
    }

    fn get_contents(&self, retval: &mut usize) -> HResult {
        *retval = self.data.contents_ptr;
        hresults::E_OK
    }

    fn get_format(&self, retval: &mut ngsbase::PixelFormat) -> HResult {
        *retval = self.data.format;
        hresults::E_OK
    }

    fn get_size(&self, retval: &mut Vector2<i32>) -> HResult {
        *retval = self.data.size;
        hresults::E_OK
    }

    /// Create an `ImageRef` from the contained `ImageData`, and wrap it with
    /// `ComImage`.
    fn into_image(&self, retval: &mut ComPtr<IUnknown>) -> HResult {
        to_hresult(|| {
            let ref data: BitmapData = self.data;

            let mut guard = data.image_data.lock().unwrap();
            let image_data = guard.take().ok_or(hresults::E_UNEXPECTED)?;

            let image_ref = ImageRef::new_immutable(image_data);
            *retval = ComImage::new(image_ref);

            Ok(())
        })
    }

    fn lock(&self) -> HResult {
        let ref data: BitmapData = self.data;

        let guard = data.image_data.lock().unwrap();

        let mut guard_cell = data.guard_cell.lock().unwrap();
        assert!(guard_cell.is_none());
        *guard_cell = Some(guard);

        hresults::E_OK
    }

    fn unlock(&self) -> HResult {
        let ref data: BitmapData = self.data;

        let mut guard_cell = data.guard_cell.lock().unwrap();
        if guard_cell.is_none() {
            return E_PF_THREAD;
        }
        *guard_cell = None;

        hresults::E_OK
    }
}

com_impl! {
    /// A COM wrapper for `ngspf::canvas::ImageRef`.
    class ComImage {
        iany: IAny;
        @data: ImageRef;
    }
}

impl ComImage {
    /// Construct a `ComImage` and get it as an `IUnknown`.
    pub fn new(image_ref: ImageRef) -> ComPtr<IUnknown> {
        Self::alloc(image_ref)
    }

    /// Get the contained `ImageRef`.
    pub fn image_ref(&self) -> &ImageRef {
        &self.data
    }
}
