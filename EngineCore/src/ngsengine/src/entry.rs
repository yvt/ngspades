//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![allow(dead_code)] // For `Engine::data`
use cgmath::Vector2;

use ngsbase::{
    INgsEngine, INgsEngineTrait, INgsPFBitmap, INgsPFFontFactory, INgsPFWorkspace,
    INgsPFWorkspaceListener, PixelFormat, PixelFormatItem,
};
use ngscom::{hresults, to_hresult, ComPtr, HResult, UnownedComPtr, com_impl};

use ngspf::canvas::{ImageData, ImageFormat};
use ngspf_com;

com_impl! {
    class Engine {
        ings_engine: INgsEngine;
        @data: EngineData;
    }
}

#[derive(Debug)]
struct EngineData {
    font_factory: ComPtr<INgsPFFontFactory>,
}

impl Engine {
    fn new() -> ComPtr<INgsEngine> {
        (&Self::alloc(EngineData {
            font_factory: (&ngspf_com::text::ComFontFactory::new()).into(),
        })).into()
    }
}

impl INgsEngineTrait for Engine {
    fn create_workspace(
        &self,
        listener: UnownedComPtr<INgsPFWorkspaceListener>,
        retval: &mut ComPtr<INgsPFWorkspace>,
    ) -> HResult {
        to_hresult(|| {
            *retval = ngspf_com::ComWorkspace::new(listener.to_owned())?;
            Ok(())
        })
    }

    fn create_bitmap(
        &self,
        size: Vector2<i32>,
        format: PixelFormat,
        retval: &mut ComPtr<INgsPFBitmap>,
    ) -> HResult {
        to_hresult(|| {
            let size = size.cast::<usize>().ok_or(hresults::E_INVALIDARG)?;
            let format = match format.get().ok_or(hresults::E_INVALIDARG)? {
                PixelFormatItem::SrgbRgba8 => ImageFormat::SrgbRgba8,
                PixelFormatItem::SrgbRgba8Premul => ImageFormat::SrgbRgba8Premul,
            };
            let image_data = ImageData::new(size, format);
            *retval = (&ngspf_com::ComBitmap::new(image_data)).into();
            Ok(())
        })
    }

    fn get_font_factory(&self, retval: &mut ComPtr<INgsPFFontFactory>) -> HResult {
        *retval = self.data.font_factory.clone();
        hresults::E_OK
    }
}

#[no_mangle]
pub unsafe extern "C" fn ngsengine_create(retval: &mut ComPtr<INgsEngine>) -> HResult {
    *retval = Engine::new();
    hresults::E_OK
}
