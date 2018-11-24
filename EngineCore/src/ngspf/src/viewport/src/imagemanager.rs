//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use iterpool::{Pool, PoolPtr};

use zangfx::base as gfx;
use zangfx::base::Result;
use zangfx::utils::{uploader, DeviceUtils};

use canvas::{ImageData, ImageRef};
use core::prelude::*;
use core::PresenterFrame;

use gfxutils::{ArgPoolSet, ArgPoolTable, HeapSet, HeapSetAlloc};

/// Manages residency of `ImageRef` on a ZanGFX device.
///
///  - First, uses of `ImageRef` are detected by the method `scan_nodes` which
///    inspects the `contents` of every presentation_cmd_pool`Layer`. Those images are added to the
///    `ImageRefTable`.
///
///  - TODO: persistent image group (manage ref-count collectively to maintain
///    the residency of a large number of images efficiently)
///
///  - If new images were found (i.e. not in the resident image set but
///    in `ImageRefTable`), they are also added to `new_images_list`.
///
///  - `upload` goes through `new_images_list` and creates an `Image` and
///    `ImageView` for each new image. It also initiates the upload using
///    `zangfx::utils::uploader::Uploader`.
///
///  - At this point the application can retrieve `Image`s and `ImageView`s
///    using the `get` method. Since the upload might be still in progress, the
///    application must encode a fence wait operation.
///
///  - The application would track the execution state of command buffers and
///    eventualy call `release` to release images in `ImageRefTable`.
///
/// ## Argument table generation
///
/// `ImageManager` automatically creates argument tables for each image for
/// common usage.
///
/// ## Queue Mappings
///
///  - The main queue is used for layer contents because they are going
///    to be consumed by the compositor within the same frame.
///  - The copy queue is used for preloading persistent image groups so they
///    can be uploaded asynchronously. They are transferred to the main
///    queue upon first actual uses.
///
#[derive(Debug)]
pub struct ImageManager {
    device: gfx::DeviceRef,
    uploader: uploader::Uploader,
    image_heap: HeapSet,
    images: Pool<Image>,
    image_map: HashMap<ImageRef, PoolPtr>,

    arg_pool_set: ArgPoolSet,
    samplers: [gfx::SamplerRef; 2],
    white_image: gfx::ImageRef,
    white_image_arg_pool_table: ArgPoolTable,

    unused_image_list: Vec<PoolPtr>,
    new_images_list: Vec<PoolPtr>,
}

/// A table of references to images managed by `ImageManager`.
#[derive(Debug)]
pub struct ImageRefTable {
    image_ptrs: HashSet<PoolPtr>,
}

#[derive(Debug)]
pub struct ResidentImage<'a> {
    data: &'a ResidentImageData,
}

#[derive(Debug)]
struct Image {
    resident: Option<ResidentImageData>,
    image_ref: ImageRef,
    /// The number of `ImageRefTable`s referencing this image. An reference
    /// from `ImageManager::new_images_list` is also included.
    ref_count: usize,
}

#[derive(Debug)]
struct ResidentImageData {
    image: gfx::ImageRef,
    arg_pool_table: [ArgPoolTable; 2],
    alloc: HeapSetAlloc,
    session_id: uploader::SessionId,
}

fn bytes_of_image(image_ref: &ImageRef, frame: &PresenterFrame) -> usize {
    image_ref
        .image_data()
        .get_presenter_ref(frame)
        .unwrap()
        .pixels_u32()
        .len()
        * 4
}

fn uncommit_image(heap_set: &mut HeapSet, resident_image: &ResidentImageData) -> Result<()> {
    heap_set.unbind(&resident_image.alloc, (&resident_image.image).into())?;
    for e in &resident_image.arg_pool_table {
        e.0.destroy_tables(&[&e.1])?;
    }
    Ok(())
}

impl Drop for ImageManager {
    fn drop(&mut self) {
        for (_, image_ptr) in self.image_map.drain() {
            let mut image = self.images.deallocate(image_ptr).unwrap();
            if let Some(resident) = image.resident.take() {
                uncommit_image(&mut self.image_heap, &resident).unwrap();
            }
        }
    }
}

impl ImageManager {
    /// Consturct an `ImageManager`.
    ///
    /// `table_sig` is an argument table signature for `ARG_TABLE_CONTENTS`.
    /// `samplers` and `white_image` are used to prefill argument tables.
    pub fn new(
        device: &gfx::DeviceRef,
        main_queue: &gfx::CmdQueueRef,
        table_sig: gfx::ArgTableSigRef,
        samplers: [gfx::SamplerRef; 2],
        white_image: gfx::ImageRef,
    ) -> Result<Self> {
        let uploader = uploader::Uploader::new(uploader::UploaderParams {
            device: device.clone(),
            queue: main_queue.clone(),
            max_bytes_per_session: 8_000_000,
            max_bytes_ongoing: 32_000_000,
        })?;

        let image_memory_type = device
            .try_choose_memory_type_private(gfx::ImageFormat::SrgbRgba8)?
            .unwrap();

        let mut arg_pool_set = ArgPoolSet::new(device.clone(), table_sig)?;

        use super::compositor::composite;
        let white_image_arg_pool_table = arg_pool_set.new_table()?;
        device.update_arg_table(
            arg_pool_set.table_sig(),
            &white_image_arg_pool_table.0,
            &white_image_arg_pool_table.1,
            &[
                (composite::ARG_C_IMAGE, 0, [&white_image][..].into()),
                (composite::ARG_C_IMAGE_SAMPLER, 0, [&samplers[0]][..].into()),
                (composite::ARG_C_MASK, 0, [&white_image][..].into()),
                (composite::ARG_C_MASK_SAMPLER, 0, [&samplers[0]][..].into()),
            ],
        )?;

        Ok(Self {
            image_heap: HeapSet::new(device, image_memory_type),
            device: device.clone(),
            uploader,
            images: Pool::new(),
            image_map: HashMap::new(),

            arg_pool_set,
            samplers,
            white_image,
            white_image_arg_pool_table,

            unused_image_list: Vec::new(),
            new_images_list: Vec::new(),
        })
    }

    pub fn white_image_arg_pool_table(&self) -> (&gfx::ArgPoolRef, &gfx::ArgTableRef) {
        let ArgPoolTable(pool, table) = &self.white_image_arg_pool_table;
        (pool, table)
    }

    pub fn uploader_mut(&mut self) -> &mut uploader::Uploader {
        &mut self.uploader
    }

    pub fn new_ref_table(&self) -> ImageRefTable {
        ImageRefTable {
            image_ptrs: HashSet::new(),
        }
    }

    /// Release images and update `Image::ref_count` and
    /// `unused_image_list` accordingly. Calling this drains the
    /// `ImageRefTable`.
    pub fn release(&mut self, ref_table: &mut ImageRefTable) -> Result<()> {
        for image_ptr in ref_table.image_ptrs.drain() {
            let ref mut image = self.images[image_ptr];
            image.ref_count -= 1;
            if image.ref_count == 0 {
                self.unused_image_list.push(image_ptr);
            }
        }

        for image_ptr in self.unused_image_list.drain(..) {
            let mut image = self.images.deallocate(image_ptr).unwrap();
            self.image_map.remove(&image.image_ref);
            if let Some(resident) = image.resident.take() {
                uncommit_image(&mut self.image_heap, &resident)?;
            }
        }

        Ok(())
    }

    /// Add an image reference to the `ImageRefTable` and queue the image for
    /// uploading.
    ///
    /// The image might not be available immediately. You must call `upload`
    /// first at least once before calling `get`.
    pub fn use_image(&mut self, image_ref: &ImageRef, ref_table: &mut ImageRefTable) {
        if let Some(&image_ptr) = self.image_map.get(image_ref) {
            // The image is already queued for uploading, or being uploaded
            if ref_table.image_ptrs.contains(&image_ptr) {
                return;
            }
            ref_table.image_ptrs.insert(image_ptr);
            self.images[image_ptr].ref_count += 1;
        } else {
            // New image
            let image = Image {
                resident: None,
                image_ref: ImageRef::clone(image_ref),
                // `ImageRefTable` + `ImageManager::new_images_list`
                ref_count: 2,
            };
            let image_ptr = self.images.allocate(image);
            ref_table.image_ptrs.insert(image_ptr);
            self.image_map.insert(ImageRef::clone(image_ref), image_ptr);
            self.new_images_list.push(image_ptr);
        }
    }

    /// Initiate the upload of queued images.
    pub fn upload(&mut self, frame: &PresenterFrame) -> Result<uploader::SessionId> {
        if self.new_images_list.len() == 0 {
            return Ok(0);
        }

        // Create image objects
        let ref device = self.device;
        let ref mut images = self.images;
        let gfx_images: Result<Vec<_>> = self
            .new_images_list
            .iter()
            .map(|&image_ptr| {
                let ref mut image: Image = images[image_ptr];

                assert!(image.resident.is_none());

                let image_data = image.image_ref.image_data();
                let image_data: &ImageData = image_data.get_presenter_ref(frame).unwrap();

                let gfx_image = device
                    .build_image()
                    .extents(&image_data.size().cast::<u32>().unwrap()[0..2])
                    .format(gfx::ImageFormat::SrgbRgba8)
                    .usage(flags![gfx::ImageUsageFlags::{CopyWrite | Sampled}])
                    .build()?;

                Ok(gfx_image)
            }).collect();
        let gfx_images = gfx_images?;

        // Allocate a heap to hold all those images
        fn to_res<'a>(x: &'a gfx::ImageRef) -> gfx::ResourceRef<'a> {
            x.into()
        }
        let allocs = self.image_heap.bind_multi(gfx_images.iter().map(to_res))?;

        // Store the allocated image (view) objects
        for ((&image_ptr, alloc), gfx_image) in self
            .new_images_list
            .iter()
            .zip(allocs.into_iter())
            .zip(gfx_images.into_iter())
        {
            let ref mut image: Image = images[image_ptr];

            let arg_pool_table = [
                self.arg_pool_set.new_table()?,
                self.arg_pool_set.new_table()?,
            ];
            use super::compositor::composite;

            for i in 0..2 {
                self.device.update_arg_table(
                    &self.arg_pool_set.table_sig(),
                    &arg_pool_table[i].0,
                    &arg_pool_table[i].1,
                    &[
                        (composite::ARG_C_IMAGE, 0, [&gfx_image][..].into()),
                        (
                            composite::ARG_C_IMAGE_SAMPLER,
                            0,
                            [&self.samplers[i]][..].into(),
                        ),
                        (composite::ARG_C_MASK, 0, [&self.white_image][..].into()),
                        (
                            composite::ARG_C_MASK_SAMPLER,
                            0,
                            [&self.samplers[i]][..].into(),
                        ),
                    ],
                )?;
            }

            image.resident = Some(ResidentImageData {
                image: gfx_image,
                alloc,
                arg_pool_table,
                session_id: 0, // set later
            });
        }

        // Initiate the upload
        struct UploadRequest<'a> {
            frame: &'a PresenterFrame,
            image: &'a Image,
        }
        impl<'a> uploader::UploadRequest for UploadRequest<'a> {
            fn size(&self) -> usize {
                bytes_of_image(&self.image.image_ref, self.frame)
            }

            fn populate(&self, staging_buffer: &mut [u8]) {
                use std::ptr::copy;
                let image_data = self.image.image_ref.image_data();
                let image_data: &ImageData = image_data.get_presenter_ref(self.frame).unwrap();
                let pixels = image_data.pixels_u32();
                let size = pixels.len() * 4;
                assert!(size <= staging_buffer.len());
                unsafe {
                    copy(
                        pixels.as_ptr() as *const u8,
                        staging_buffer.as_mut_ptr(),
                        size,
                    );
                }
            }

            fn copy(
                &self,
                encoder: &mut gfx::CopyCmdEncoder,
                staging_buffer: &gfx::BufferRef,
                staging_buffer_range: Range<gfx::DeviceSize>,
            ) -> Result<()> {
                let image_data = self.image.image_ref.image_data();
                let image_data: &ImageData = image_data.get_presenter_ref(self.frame).unwrap();

                let size = image_data.size();

                let resident: &ResidentImageData = self.image.resident.as_ref().unwrap();

                encoder.copy_buffer_to_image(
                    staging_buffer,
                    &gfx::BufferImageRange {
                        offset: staging_buffer_range.start,
                        row_stride: size.x as gfx::DeviceSize,
                        plane_stride: 0,
                    },
                    &resident.image,
                    gfx::ImageAspect::Color,
                    &gfx::ImageLayerRange {
                        mip_level: 0,
                        layers: 0..1,
                    },
                    &[0, 0],
                    &size.cast::<u32>().unwrap()[0..2],
                );

                Ok(())
            }
        }
        let session_id = self
            .uploader
            .upload(self.new_images_list.iter().map(&|&image_ptr| {
                let ref image: Image = images[image_ptr];
                UploadRequest { frame, image }
            }))?;

        for image_ptr in self.new_images_list.drain(..) {
            let ref mut image: Image = images[image_ptr];
            image.resident.as_mut().unwrap().session_id = session_id;

            // The image was removed from `new_images_list` so decrement the
            // ref count
            image.ref_count -= 1;
            if image.ref_count == 0 {
                self.unused_image_list.push(image_ptr);
            }
        }

        for image_ptr in self.unused_image_list.drain(..) {
            let mut image = images.deallocate(image_ptr).unwrap();
            self.image_map.remove(&image.image_ref);
            if let Some(resident) = image.resident.take() {
                uncommit_image(&mut self.image_heap, &resident)?;
            }
        }

        Ok(session_id)
    }

    pub fn get(&self, image_ref: &ImageRef) -> Option<ResidentImage> {
        self.image_map.get(image_ref).map(|&i| ResidentImage {
            data: self.images[i]
                .resident
                .as_ref()
                .expect("The image is not queued for uploading yet"),
        })
    }

    pub fn get_fence_for_session(&self, session_id: uploader::SessionId) -> Option<&gfx::FenceRef> {
        self.uploader.get_fence(session_id)
    }
}

impl<'a> ResidentImage<'a> {
    pub fn image(&self) -> &'a gfx::ImageRef {
        &self.data.image
    }

    pub fn arg_pool_table(
        &self,
        sampler_type: usize,
    ) -> (&'a gfx::ArgPoolRef, &'a gfx::ArgTableRef) {
        let ArgPoolTable(pool, table) = &self.data.arg_pool_table[sampler_type];
        (pool, table)
    }

    #[allow(dead_code)]
    pub fn session_id(&self) -> uploader::SessionId {
        self.data.session_id
    }
}
