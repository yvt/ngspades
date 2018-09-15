//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#![allow(dead_code)] // For `ProcessorInfo::data`
use ngsbase::{INgsProcessorInfo, INgsProcessorInfoTrait};
use ngscom::{hresults, BString, BStringRef, ComPtr, HResult, com_impl};
use crate::ProcessorInfoCommon;

com_impl! {
    class ProcessorInfo {
        iprocessor_info: INgsProcessorInfo;
        @data: ProcessorInfoData;
    }
}

#[derive(Debug)]
struct ProcessorInfoData {
    common: ProcessorInfoCommon,
}

impl ProcessorInfo {
    pub fn new() -> ComPtr<INgsProcessorInfo> {
        (&Self::alloc(ProcessorInfoData {
            common: ProcessorInfoCommon::new(),
        })).into()
    }
}

impl INgsProcessorInfoTrait for ProcessorInfo {
    fn get_architecture(&self, retval: &mut BStringRef) -> HResult {
        self.data.common.get_architecture(retval)
    }

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
