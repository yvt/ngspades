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
use smallvec::SmallVec;

use base::{arg, device, handles, shader, ArgArrayIndex, ArgIndex};
use common::Result;

use arg::ArgSize;
use utils::{nil_error, OCPtr};
use spirv_cross::{ExecutionModel, ResourceBinding, SpirV2Msl};

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
    len: ArgSize,
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

    /// Used by `arg::table::ArgLayoutInfo` to estimate the overhead of the
    /// argument buffer. Not optimized because the intention is that
    /// `ArgLayoutInfo` is computed only once for each `Device` created.
    pub(super) fn encoded_size(&mut self) -> Result<ArgSize> {
        use base::arg::ArgTableSigBuilder;
        let gfx_sig = self.build()?;
        let sig: &ArgTableSig = gfx_sig.downcast_ref().unwrap();
        let size = sig.encoded_size();
        let align = sig.encoded_alignment();
        Ok((size + align - 1) & !(align - 1))
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
        let mut current_index = 0usize;

        for (_, arg_sig_builder) in self.args.iter().enumerate() {
            arg_sigs.push(ArgSig {
                index: current_index,
            });

            if let &Some(ref arg_sig_builder) = arg_sig_builder {
                // Allocate Metal argument locations for the current argument,
                // starting from `current_index` through `current_index + len - 1`.
                let metal_desc = OCPtr::new(metal::MTLArgumentDescriptor::new())
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
                current_index += arg_sig_builder.len as usize;
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
        self.len = x as _;
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
    metal_device: metal::MTLDevice,
    args: Vec<ArgSig>,
    metal_args_array: OCPtr<metal::NSArray<metal::MTLArgumentDescriptor>>,

    /// Shared instnace of `MTLArgumentEncoder`. It is not thread-safe by itself
    /// so we might have to create a temporary instance (slow!) in contended
    /// case.
    metal_arg_encoder: Mutex<OCPtr<metal::MTLArgumentEncoder>>,

    size: ArgSize,
    alignment: ArgSize,
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
        use std::cmp::max;
        let metal_arg_encoder = new_metal_arg_encoder(metal_device, *metal_args_array)?;

        let data = ArgTableSigData {
            metal_device,
            args,
            metal_args_array,
            size: metal_arg_encoder.encoded_length() as ArgSize,
            // Constant buffers must be aligned to 256 bytes in macOS.
            alignment: max(metal_arg_encoder.alignment() as ArgSize, 256),
            metal_arg_encoder: Mutex::new(metal_arg_encoder),
        };

        Ok(Self {
            data: Arc::new(data),
        })
    }

    /// Obtain a `MTLArgumentEncoder` that can safely be used in the current
    /// thread.
    fn lock_encoder<T, R>(&self, cb: T) -> Result<R>
    where
        T: FnOnce(&OCPtr<metal::MTLArgumentEncoder>) -> R,
    {
        if let Some(encoder) = self.data.metal_arg_encoder.try_lock() {
            Ok(cb(&encoder))
        } else {
            let metal_arg_encoder = unsafe {
                new_metal_arg_encoder(self.data.metal_device, *self.data.metal_args_array)?
            };
            Ok(cb(&metal_arg_encoder))
        }
    }

    pub(crate) fn encoded_size(&self) -> ArgSize {
        self.data.size
    }

    pub(crate) fn encoded_alignment(&self) -> ArgSize {
        self.data.alignment
    }

    pub(crate) fn update_arg_tables(
        &self,
        updates: &[(&handles::ArgTable, &[device::ArgUpdateSet])],
    ) -> Result<()> {
        use base::handles::ArgSlice::*;
        use arg::table::ArgTable;
        use buffer::Buffer;
        use sampler::Sampler;

        self.lock_encoder(|encoder| {
            for &(table, update_sets) in updates.iter() {
                let table: &ArgTable = table.downcast_ref().expect("bad argument table type");
                encoder.set_argument_buffer(table.metal_buffer(), table.offset() as _);

                for &(arg_index, start, resources) in update_sets.iter() {
                    // The current Metal argument index.
                    let mut index = self.data.args[arg_index].index + start;

                    // Before passing `ArgSlice` to `MTLArgumentEncoder`, it must
                    // first be converted to a slice containing Metal objects.
                    // In order to avoid heap allocation, we split the `ArgSlice`
                    // into chunks and process each chunk on a fixed size
                    // stack-allocated array (`SmallVec`).
                    match resources {
                        ImageView(_) => unimplemented!(),

                        Buffer(objs) => for objs in objs.chunks(64) {
                            let metal_objs: SmallVec<[_; 64]> = objs.iter()
                                .map(|&(_, obj)| {
                                    let my_obj: &Buffer =
                                        obj.downcast_ref().expect("bad buffer type");
                                    my_obj.metal_buffer()
                                })
                                .collect();

                            let offsets: SmallVec<[_; 64]> = objs.iter()
                                .map(|&(ref range, _)| range.start as _)
                                .collect();

                            encoder.set_buffers(
                                metal_objs.as_slice(),
                                offsets.as_slice(),
                                index as _,
                            );

                            index += objs.len();
                        },

                        Sampler(objs) => for objs in objs.chunks(64) {
                            let metal_objs: SmallVec<[_; 64]> = objs.iter()
                                .map(|obj| {
                                    let my_obj: &Sampler =
                                        obj.downcast_ref().expect("bad sampler type");
                                    my_obj.metal_sampler()
                                })
                                .collect();

                            encoder.set_sampler_states(metal_objs.as_slice(), index as _);

                            index += objs.len();
                        },
                    }
                    // Updating an `ArgUpdateSet` is done
                }
                // Updating `table` is done
            }
            // All done
        })
    }

    pub(crate) fn setup_spirv2msl(
        &self,
        s2m: &mut SpirV2Msl,
        desc_set: u32,
        msl_arg_buffer: Option<u32>,
        stage: ExecutionModel,
    ) {
        for (i, arg) in self.data.args.iter().enumerate() {
            s2m.bind_resource(&ResourceBinding {
                desc_set,
                binding: i as u32,
                msl_buffer: Some(arg.index as u32),
                msl_texture: Some(arg.index as u32),
                msl_sampler: Some(arg.index as u32),
                msl_arg_buffer,
                stage,
            });
        }
    }
}
