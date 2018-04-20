//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![allow(dead_code)] // For `Engine::data`
use cgmath::Vector2;

use ngsbase::{IBitmap, IEngine, IEngineTrait, IWorkspace, PixelFormat};
use ngscom::{hresults, to_hresult, ComPtr, HResult};

use ngspf_com;

com_impl! {
    class Engine {
        iengine: IEngine;
        @data: EngineData;
    }
}

#[derive(Debug)]
struct EngineData;

impl Engine {
    fn new() -> ComPtr<IEngine> {
        (&Self::alloc(EngineData)).into()
    }
}

impl IEngineTrait for Engine {
    fn create_workspace(&self, retval: &mut ComPtr<IWorkspace>) -> HResult {
        to_hresult(|| {
            *retval = ngspf_com::ComWorkspace::new()?;
            Ok(())
        })
    }

    fn create_bitmap(
        &self,
        _size: Vector2<i32>,
        _format: PixelFormat,
        _retval: &mut ComPtr<IBitmap>,
    ) -> HResult {
        hresults::E_NOTIMPL
    }
}

#[no_mangle]
pub unsafe extern "C" fn ngsengine_create(retval: &mut ComPtr<IEngine>) -> HResult {
    *retval = Engine::new();
    hresults::E_OK
}
