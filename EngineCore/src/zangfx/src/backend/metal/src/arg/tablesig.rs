//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `ArgTableSig` for Metal.
use arrayvec::ArrayVec;
use cocoa::base::nil;
use cocoa::foundation::NSArray;
use parking_lot::Mutex;
use std::sync::Arc;
use zangfx_metal_rs as metal;

use zangfx_base::Result;
use zangfx_base::{self as base, arg, device, shader, ArgArrayIndex, ArgIndex};
use zangfx_base::{zangfx_impl_handle, zangfx_impl_object};

use crate::arg::ArgSize;
use crate::utils::{nil_error, OCPtr};
use zangfx_spirv_cross::{ExecutionModel, IndirectArgument, ResourceBinding, SpirV2Msl};

/// Implementation of `ArgTableSigBuilder` for Metal.
#[derive(Debug)]
pub struct ArgTableSigBuilder {
    metal_device: OCPtr<metal::MTLDevice>,
    args: Vec<Option<ArgSigBuilder>>,
}

zangfx_impl_object! { ArgTableSigBuilder: dyn arg::ArgTableSigBuilder, dyn crate::Debug }

unsafe impl Send for ArgTableSigBuilder {}
unsafe impl Sync for ArgTableSigBuilder {}

#[derive(Debug, Clone)]
struct ArgSigBuilder {
    ty: arg::ArgType,
    len: ArgSize,
    image_aspect: base::ImageAspect,
}

zangfx_impl_object! { ArgSigBuilder: dyn arg::ArgSig, dyn crate::Debug }

impl ArgTableSigBuilder {
    /// Construct an `ArgTableSigBuilder`.
    ///
    /// It's up to the caller to make sure `metal_device` is valid.
    pub unsafe fn new(metal_device: metal::MTLDevice) -> Self {
        Self {
            metal_device: OCPtr::new(metal_device).expect("nil device"),
            args: Vec::new(),
        }
    }

    /// Used by `arg::table::ArgLayoutInfo` to estimate the overhead of the
    /// argument buffer. Not optimized because the intention is that
    /// `ArgLayoutInfo` is computed only once for each `Device` created.
    pub(super) fn encoded_size(&mut self) -> Result<ArgSize> {
        use zangfx_base::arg::ArgTableSigBuilder;
        let gfx_sig = self.build()?;
        let sig: &ArgTableSig = gfx_sig.downcast_ref().unwrap();
        let size = sig.encoded_size();
        let align = sig.encoded_alignment();
        Ok((size + align - 1) & !(align - 1))
    }
}

impl arg::ArgTableSigBuilder for ArgTableSigBuilder {
    fn arg(&mut self, index: ArgIndex, ty: arg::ArgType) -> &mut dyn arg::ArgSig {
        if self.args.len() <= index {
            self.args.resize(index + 1, None);
        }

        self.args[index] = Some(ArgSigBuilder::new(ty));

        self.args[index].as_mut().unwrap()
    }

    fn build(&mut self) -> Result<arg::ArgTableSigRef> {
        let mut metal_args = Vec::with_capacity(self.args.len());
        let mut arg_sigs = Vec::with_capacity(self.args.len());
        let mut current_index = 0usize;

        for (_, arg_sig_builder) in self.args.iter().enumerate() {
            if let &Some(ref arg_sig_builder) = arg_sig_builder {
                arg_sigs.push(Some(ArgSig {
                    index: current_index,
                    ty: arg_sig_builder.ty,
                    image_aspect: arg_sig_builder.image_aspect,
                }));

                // Allocate Metal argument locations for the current argument,
                // starting from `current_index` through `current_index + len - 1`.
                let metal_desc = OCPtr::new(metal::MTLArgumentDescriptor::new())
                    .ok_or_else(|| nil_error("MTLArgumentDescriptor argumentDescriptor"))?;

                metal_desc.set_index(current_index as _);
                metal_desc.set_array_length(arg_sig_builder.len as _);

                use zangfx_base::arg::ArgType::*;
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
            } else {
                arg_sigs.push(None);
            }
        }

        use std::mem::transmute;
        let metal_args_array: OCPtr<metal::NSArray<_>> = unsafe {
            let ns_array = NSArray::arrayWithObjects(nil, transmute(metal_args.as_slice()));
            OCPtr::new(transmute(ns_array)).ok_or_else(|| nil_error("NSArray arrayWithObjects"))?
        };

        unsafe { ArgTableSig::new(*self.metal_device, metal_args_array, arg_sigs) }
            .map(arg::ArgTableSigRef::new)
    }
}

impl ArgSigBuilder {
    fn new(ty: arg::ArgType) -> Self {
        Self {
            ty,
            len: 1,
            image_aspect: base::ImageAspect::Color,
        }
    }
}

impl arg::ArgSig for ArgSigBuilder {
    fn set_len(&mut self, x: ArgArrayIndex) -> &mut dyn arg::ArgSig {
        self.len = x as _;
        self
    }

    fn set_stages(&mut self, _: shader::ShaderStageFlags) -> &mut dyn arg::ArgSig {
        self
    }

    fn set_image_aspect(&mut self, v: base::ImageAspect) -> &mut dyn arg::ArgSig {
        self.image_aspect = v;
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

zangfx_impl_handle! { ArgTableSig, arg::ArgTableSigRef }

#[derive(Debug)]
struct ArgTableSigData {
    metal_device: OCPtr<metal::MTLDevice>,
    args: Vec<Option<ArgSig>>,
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
    ty: arg::ArgType,

    /// The starting index of the argument in an argument buffer.
    index: usize,

    image_aspect: base::ImageAspect,
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
        args: Vec<Option<ArgSig>>,
    ) -> Result<Self> {
        use std::cmp::max;
        let metal_arg_encoder = new_metal_arg_encoder(metal_device, *metal_args_array)?;

        let data = ArgTableSigData {
            metal_device: OCPtr::new(metal_device).expect("nil device"),
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
                new_metal_arg_encoder(*self.data.metal_device, *self.data.metal_args_array)?
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
        updates: &[(
            (&arg::ArgPoolRef, &arg::ArgTableRef),
            &[device::ArgUpdateSet<'_>],
        )],
    ) -> Result<()> {
        use crate::arg::table::ArgTable;
        use crate::buffer::Buffer;
        use crate::image::Image;
        use crate::sampler::Sampler;
        use std::raw::TraitObject;
        use zangfx_base::ArgSlice::*;

        let mut metal_textures: ArrayVec<[_; 64]> = ArrayVec::new();
        let mut metal_buffers: ArrayVec<[_; 64]> = ArrayVec::new();
        let mut offsets: ArrayVec<[_; 64]> = ArrayVec::new();
        let mut metal_samplers: ArrayVec<[_; 64]> = ArrayVec::new();

        self.lock_encoder(|encoder| {
            let mut last_pool = std::ptr::null();
            let mut metal_buffer = metal::MTLBuffer::nil();

            for &((pool, table), update_sets) in updates.iter() {
                let table: &ArgTable = table.downcast_ref().expect("bad argument table type");

                // We'd like to reduce the number of `ArgTable::metal_buffer`
                // because it leads to at least two indirect calls, inhibiting
                // compiler optimization
                let pool_to: TraitObject = unsafe { std::mem::transmute(&**pool) };
                let pool_ptr: *const () = pool_to.data;
                if pool_ptr != last_pool {
                    last_pool = pool_ptr;
                    metal_buffer = table.metal_buffer(pool);
                }

                encoder.set_argument_buffer(metal_buffer, table.offset() as _);

                for &(arg_index, start, resources) in update_sets.iter() {
                    // The current Metal argument index.
                    let mut index = self.data.args[arg_index].as_ref().unwrap().index + start;

                    // Before passing `ArgSlice` to `MTLArgumentEncoder`, it must
                    // first be converted to a slice containing Metal objects.
                    // In order to avoid heap allocation, we split the `ArgSlice`
                    // into chunks and process each chunk on a fixed size
                    // stack-allocated array (`ArrayVec`).
                    match resources {
                        Image(objs) => {
                            for objs in objs.chunks(64) {
                                metal_textures.extend(objs.iter().map(|obj| {
                                    let my_obj: &Image =
                                        obj.downcast_ref().expect("bad image view type");
                                    my_obj.metal_texture()
                                }));

                                encoder.set_textures(metal_textures.as_slice(), index as _);
                                metal_textures.clear();

                                index += objs.len();
                            }
                        }

                        Buffer(objs) => {
                            for objs in objs.chunks(64) {
                                metal_buffers.extend(objs.iter().map(|&(_, obj)| {
                                    let my_obj: &Buffer =
                                        obj.downcast_ref().expect("bad buffer type");
                                    let (metal_buffer, _) =
                                        my_obj.metal_buffer_and_offset().unwrap();
                                    metal_buffer
                                }));

                                offsets.extend(objs.iter().map(|&(ref range, obj)| {
                                    let my_obj: &Buffer =
                                        obj.downcast_ref().expect("bad buffer type");
                                    let (_, offset) = my_obj.metal_buffer_and_offset().unwrap();
                                    range.start + offset
                                }));

                                encoder.set_buffers(
                                    metal_buffers.as_slice(),
                                    offsets.as_slice(),
                                    index as _,
                                );
                                metal_buffers.clear();
                                offsets.clear();

                                index += objs.len();
                            }
                        }

                        Sampler(objs) => {
                            for objs in objs.chunks(64) {
                                metal_samplers.extend(objs.iter().map(|obj| {
                                    let my_obj: &Sampler =
                                        obj.downcast_ref().expect("bad sampler type");
                                    my_obj.metal_sampler()
                                }));

                                encoder.set_sampler_states(metal_samplers.as_slice(), index as _);
                                metal_samplers.clear();

                                index += objs.len();
                            }
                        }
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
        msl_arg_buffer: u32,
        stage: ExecutionModel,
    ) {
        for (i, arg) in self.data.args.iter().enumerate() {
            if let &Some(ref arg) = arg {
                s2m.bind_resource(&ResourceBinding {
                    desc_set,
                    binding: i as u32,
                    msl_buffer: Some(arg.index as u32),
                    msl_texture: Some(arg.index as u32),
                    msl_sampler: Some(arg.index as u32),
                    msl_arg_buffer: Some(msl_arg_buffer),
                    stage,
                    is_depth_texture: arg.image_aspect == base::ImageAspect::Depth,
                });

                // Since each indirect argument is given a binding location in a way
                // resembling those of Vulkan's descriptors, you might be lulled
                // into a false impression that you don't have to declare them in
                // MSL provided that they are not statically used by the shader.
                // That's not true. The physical location of each argument within an
                // argument buffer is determined by the fields defined in the
                // argument buffer. You have to define every field of an argument
                // buffer in every shader that accesses the same argument buffer.
                let msl_type = match arg.ty {
                    arg::ArgType::StorageBuffer => "device int *",
                    arg::ArgType::UniformBuffer => "constant int *",
                    // TODO: texture type? array types?
                    arg::ArgType::SampledImage => "texture2d<float>",
                    arg::ArgType::StorageImage => "texture2d<float, access::read>",
                    arg::ArgType::Sampler => "sampler",
                };
                s2m.add_indirect_argument(&IndirectArgument {
                    msl_arg_buffer,
                    msl_arg: i as u32,
                    msl_type,
                });
            }
        }
    }
}
