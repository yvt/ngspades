//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `ArgTableSig` for Metal.
use metal;
use cocoa::foundation::NSArray;
use cocoa::base::nil;
use parking_lot::Mutex;
use std::sync::Arc;

use base::{arg, handles, shader, ArgArrayIndex, ArgIndex};
use common::Result;

use utils::{nil_error, OCPtr};

/// Implementation of `ArgTableSigBuilder` for Metal.
#[derive(Debug)]
pub struct ArgTableSigBuilder {
    /// A reference to a `MTLDevice`. We are not required to maintain a strong
    /// reference. (See the base interface's documentation)
    metal_device: metal::MTLDevice,
    args: Vec<Option<ArgSigBuilder>>,
}

zangfx_impl_object! { ArgTableSigBuilder: arg::ArgTableSigBuilder, ::Debug }

unsafe impl Send for ArgTableSigBuilder {}
unsafe impl Sync for ArgTableSigBuilder {}

#[derive(Debug, Clone)]
struct ArgSigBuilder {
    ty: arg::ArgType,
    len: usize,
}

zangfx_impl_object! { ArgSigBuilder: arg::ArgSig, ::Debug }

impl ArgTableSigBuilder {
    /// Construct an `ArgTableSigBuilder`.
    ///
    /// Ir's up to the caller to maintain the lifetime of `metal_device`.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device,
            args: Vec::new(),
        }
    }
}

impl arg::ArgTableSigBuilder for ArgTableSigBuilder {
    fn arg(&mut self, index: ArgIndex, ty: arg::ArgType) -> &mut arg::ArgSig {
        if self.args.len() <= index {
            self.args.resize(index + 1, None);
        }

        self.args[index] = Some(ArgSigBuilder { ty, len: 1 });

        self.args[index].as_mut().unwrap()
    }

    fn build(&mut self) -> Result<handles::ArgTableSig> {
        let mut metal_args = Vec::with_capacity(self.args.len());
        let mut arg_sigs = Vec::with_capacity(self.args.len());
        let mut current_index = 0;

        for (_, arg_sig_builder) in self.args.iter().enumerate() {
            arg_sigs.push(ArgSig {
                index: current_index,
            });

            if let &Some(ref arg_sig_builder) = arg_sig_builder {
                let metal_desc = unsafe { OCPtr::from_raw(metal::MTLArgumentDescriptor::new()) }
                    .ok_or_else(|| nil_error("MTLArgumentDescriptor argumentDescriptor"))?;

                metal_desc.set_index(current_index as _);
                metal_desc.set_array_length(arg_sig_builder.len as _);

                use base::arg::ArgType::*;
                match arg_sig_builder.ty {
                    StorageImage | SampledImage => {
                        metal_desc.set_data_type(metal::MTLDataType::Texture);
                        metal_desc.set_access(metal::MTLArgumentAccess::ReadOnly);
                    }
                    Sampler => {
                        metal_desc.set_data_type(metal::MTLDataType::Sampler);
                        metal_desc.set_access(metal::MTLArgumentAccess::ReadOnly);
                    }
                    UniformBuffer => {
                        metal_desc.set_data_type(metal::MTLDataType::Pointer);
                        metal_desc.set_access(metal::MTLArgumentAccess::ReadOnly);
                    }
                    StorageBuffer => {
                        metal_desc.set_data_type(metal::MTLDataType::Pointer);
                        metal_desc.set_access(metal::MTLArgumentAccess::ReadWrite);
                    }
                }

                metal_args.push(metal_desc);
                current_index += arg_sig_builder.len;
            }
        }

        use std::mem::transmute;
        let metal_args_array: OCPtr<metal::NSArray<_>> = unsafe {
            let ns_array = NSArray::arrayWithObjects(nil, transmute(metal_args.as_slice()));
            OCPtr::new(transmute(ns_array)).ok_or_else(|| nil_error("NSArray arrayWithObjects"))?
        };

        unsafe { ArgTableSig::new(self.metal_device, metal_args_array, arg_sigs) }
            .map(handles::ArgTableSig::new)
    }
}

impl arg::ArgSig for ArgSigBuilder {
    fn set_len(&mut self, x: ArgArrayIndex) -> &mut arg::ArgSig {
        self.len = x;
        self
    }

    fn set_stages(&mut self, _: shader::ShaderStageFlags) -> &mut arg::ArgSig {
        self
    }
}

/// Implementation of `ArgTableSig` for Metal.
#[derive(Debug, Clone)]
pub struct ArgTableSig {
    data: Arc<ArgTableSigData>,
}

unsafe impl Send for ArgTableSig {}
unsafe impl Sync for ArgTableSig {}

zangfx_impl_handle! { ArgTableSig, handles::ArgTableSig }

#[derive(Debug)]
struct ArgTableSigData {
    args: Vec<ArgSig>,
    metal_args_array: OCPtr<metal::NSArray<metal::MTLArgumentDescriptor>>,

    /// Shared instnace of `MTLArgumentEncoder`. It is not thread-safe by itself
    /// so we might have to create a temporary instance (slow!) in contended
    /// case.
    metal_arg_encoder: Mutex<OCPtr<metal::MTLArgumentEncoder>>,
}

#[derive(Debug)]
struct ArgSig {
    /// The starting index of the argument in an argument buffer.
    index: usize,
}

unsafe fn new_metal_arg_encoder(
    metal_device: metal::MTLDevice,
    metal_args_array: metal::NSArray<metal::MTLArgumentDescriptor>,
) -> Result<OCPtr<metal::MTLArgumentEncoder>> {
    OCPtr::new(metal_device.new_argument_encoder_with_arguments(metal_args_array))
        .ok_or_else(|| nil_error("MTLDevice newArgumentEncoderWithArguments:"))
}

impl ArgTableSig {
    unsafe fn new(
        metal_device: metal::MTLDevice,
        metal_args_array: OCPtr<metal::NSArray<metal::MTLArgumentDescriptor>>,
        args: Vec<ArgSig>,
    ) -> Result<Self> {
        let metal_arg_encoder = new_metal_arg_encoder(metal_device, *metal_args_array)?;

        let data = ArgTableSigData {
            args,
            metal_args_array,
            metal_arg_encoder: Mutex::new(metal_arg_encoder),
        };

        Ok(Self {
            data: Arc::new(data),
        })
    }

    // TODO: update argument table
}
