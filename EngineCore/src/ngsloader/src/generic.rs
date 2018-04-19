//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![allow(dead_code)] // For `ProcessorInfo::data`
use ngsbase::{IProcessorInfo, IProcessorInfoTrait};
use ngscom::{hresults, BString, BStringRef, ComPtr, HResult};

com_impl! {
    class ProcessorInfo {
        iprocessor_info: IProcessorInfo;
        @data: ProcessorInfoData;
    }
}

#[derive(Debug)]
struct ProcessorInfoData;

impl ProcessorInfo {
    pub fn new() -> ComPtr<IProcessorInfo> {
        (&Self::alloc(ProcessorInfoData)).into()
    }
}

impl IProcessorInfoTrait for ProcessorInfo {
    fn get_vendor(&self, retval: &mut BStringRef) -> HResult {
        *retval = BStringRef::new("Unknown");
        hresults::E_OK
    }

    fn supports_feature(&self, _name: Option<&BString>, retval: &mut bool) -> HResult {
        if name.is_none() {
            return hresults::E_POINTER;
        }
        *retval = false;
        hresults::E_OK
    }
}
