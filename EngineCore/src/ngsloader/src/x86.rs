//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
extern crate raw_cpuid;
use self::raw_cpuid::CpuId;

#[cfg(target_arch = "x86")]
use std::arch::x86 as vendor;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64 as vendor;

use ngsbase::{INgsProcessorInfo, INgsProcessorInfoTrait};
use ngscom::{hresults, BString, BStringRef, ComPtr, HResult};
use ProcessorInfoCommon;

com_impl! {
    class ProcessorInfo {
        iprocessor_info: INgsProcessorInfo;
        @data: ProcessorInfoData;
    }
}

#[derive(Debug)]
struct ProcessorInfoData {
    common: ProcessorInfoCommon,
    vendor: String,
    mmx: bool,
    sse: bool,
    sse2: bool,
    sse3: bool,
    ssse3: bool,
    avx: bool,
    avx2: bool,
    fma3: bool,
}

#[target_feature(enable = "xsave")]
unsafe fn x86_get_xcr(xcr_no: u32) -> u64 {
    vendor::_xgetbv(xcr_no)
}

impl ProcessorInfo {
    pub fn new() -> ComPtr<INgsProcessorInfo> {
        let cpuid = CpuId::new();

        let feature_info = cpuid.get_feature_info();
        let feature_info = feature_info.as_ref();

        let extended_feature_info = CpuId::new().get_extended_feature_info();
        let extended_feature_info = extended_feature_info.as_ref();

        let vendor = cpuid
            .get_vendor_info()
            .map(|x| x.as_string().to_owned())
            .unwrap_or_else(String::new);

        let mmx = feature_info.map(|x| x.has_mmx()).unwrap_or(false);
        let sse = feature_info.map(|x| x.has_sse()).unwrap_or(false);
        let sse2 = feature_info.map(|x| x.has_sse2()).unwrap_or(false);
        let sse3 = feature_info.map(|x| x.has_sse3()).unwrap_or(false);
        let ssse3 = feature_info.map(|x| x.has_ssse3()).unwrap_or(false);
        let avx = feature_info.map(|x| x.has_avx()).unwrap_or(false)
            && feature_info.map(|x| x.has_oxsave()).unwrap_or(false) && {
            // Must check if the operating system supports the extended state saving
            unsafe { (x86_get_xcr(0) & 0b110) == 0b110 }
        };

        let avx2 = avx && extended_feature_info.map(|x| x.has_avx2()).unwrap_or(false);
        let fma3 = avx && feature_info.map(|x| x.has_fma()).unwrap_or(false);

        (&Self::alloc(ProcessorInfoData {
            common: ProcessorInfoCommon::new(),
            vendor,
            mmx,
            sse,
            sse2,
            sse3,
            ssse3,
            avx,
            avx2,
            fma3,
        })).into()
    }
}

impl INgsProcessorInfoTrait for ProcessorInfo {
    fn get_architecture(&self, retval: &mut BStringRef) -> HResult {
        self.data.common.get_architecture(retval)
    }

    fn get_vendor(&self, retval: &mut BStringRef) -> HResult {
        *retval = BStringRef::new(&self.data.vendor);
        hresults::E_OK
    }

    fn supports_feature(&self, name: Option<&BString>, retval: &mut bool) -> HResult {
        if name.is_none() {
            return hresults::E_POINTER;
        }
        *retval = false;

        let name = name.unwrap().as_str();
        if name == "MMX" {
            *retval = self.data.mmx;
        } else if name == "SSE" {
            *retval = self.data.sse;
        } else if name == "SSE2" {
            *retval = self.data.sse2;
        } else if name == "SSE3" {
            *retval = self.data.sse3;
        } else if name == "SSSE3" {
            *retval = self.data.ssse3;
        } else if name == "AVX" {
            *retval = self.data.avx;
        } else if name == "AVX2" {
            *retval = self.data.avx2;
        } else if name == "FMA3" {
            *retval = self.data.fma3;
        }

        hresults::E_OK
    }
}
