//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::ops::Range;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};

use iterpool::{Pool, PoolPtr};

use zangfx::base as gfx;
use zangfx::prelude::*;
use zangfx::base::Result;
use zangfx::utils::{self, uploader, DeviceUtils};

use super::{ImageData, ImageRef, Layer, LayerContents};
use core::{NodeRef, PresenterFrame};
use core::prelude::*;

use gfxutils::{HeapSet, HeapSetAlloc};

/// Manages residency of `ImageRef` on a ZanGFX device.
///
///  - First, uses of `ImageRef` are detected by the method `scan_nodes` which
///    inspects the `contents` of every `Layer`. Those images are added to the
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
    device: Arc<gfx::Device>,
    uploader: uploader::Uploader,
    image_heap: HeapSet,
    images: Pool<Image>,
    image_map: HashMap<ImageRef, PoolPtr>,

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
    image: gfx::Image,
    image_view: gfx::ImageView,
    alloc: HeapSetAlloc,
    session_id: uploader::SessionId,
}

fn bytes_of_image(image_ref: &ImageRef, frame: &PresenterFrame) -> usize {
    image_ref
        .image_data()
        .get_presenter_ref(frame)
        .unwrap()
        .pixels_u32()
        .len() * 4
}

fn uncommit_image(
    device: &gfx::Device,
    heap_set: &mut HeapSet,
    resident_image: &ResidentImageData,
) -> Result<()> {
    heap_set.unbind(&resident_image.alloc);
    device.destroy_image(&resident_image.image)?;
    device.destroy_image_view(&resident_image.image_view)?;
    Ok(())
}

impl Drop for ImageManager {
    fn drop(&mut self) {
        for (_, image_ptr) in self.image_map.drain() {
            let mut image = self.images.deallocate(image_ptr).unwrap();
            if let Some(resident) = image.resident.take() {
                uncommit_image(&*self.device, &mut self.image_heap, &resident).unwrap();
            }
        }
    }
}

impl ImageManager {
    pub fn new(device: &Arc<gfx::Device>, main_queue: &Arc<gfx::CmdQueue>) -> Result<Self> {
        let uploader = uploader::Uploader::new(uploader::UploaderParams {
            device: Arc::clone(device),
            queue: Arc::clone(main_queue),
            max_bytes_per_session: 8_000_000,
            max_bytes_ongoing: 32_000_000,
        })?;

        let image_memory_type = device
            .memory_type_for_image(
                gfx::ImageFormat::SrgbRgba8,
                flags![gfx::MemoryTypeCaps::{DeviceLocal}],
                flags![gfx::MemoryTypeCaps::{}],
            )?
            .unwrap();

        Ok(Self {
            image_heap: HeapSet::new(device, image_memory_type),
            device: Arc::clone(device),
            uploader,
            images: Pool::new(),
            image_map: HashMap::new(),

            unused_image_list: Vec::new(),
            new_images_list: Vec::new(),
        })
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
                uncommit_image(&*self.device, &mut self.image_heap, &resident)?;
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

    /// Scan image uses in the specified node and call `use_image` for all found
    /// images.
    pub fn scan_nodes(
        &mut self,
        root: &NodeRef,
        frame: &PresenterFrame,
        ref_table: &mut ImageRefTable,
    ) {
        root.for_each_node(|node| {
            if let Some(layer) = node.downcast_ref::<Layer>() {
                // Scan recursively
                if let &Some(ref child) = layer.child.read_presenter(frame).unwrap() {
                    self.scan_nodes(child, frame, ref_table);
                }

                // Check the layer contents
                match layer.contents.read_presenter(frame).unwrap() {
                    &LayerContents::Image {
                        image: ref image_ref,
                        ..
                    } => {
                        self.use_image(image_ref, ref_table);
                    }
                    _ => {}
                }
            } else {
                // Ignore an unknown node type
            }
        });
    }

    /// Initiate the upload of queued images.
    pub fn upload(&mut self, frame: &PresenterFrame) -> Result<()> {
        if self.new_images_list.len() == 0 {
            return Ok(());
        }

        // Create image objects
        let ref device = self.device;
        let ref mut images = self.images;
        let gfx_images: Result<Vec<_>> = self.new_images_list
            .iter()
            .map(|&image_ptr| {
                let ref mut image: Image = images[image_ptr];

                assert!(image.resident.is_none());

                let image_data = image.image_ref.image_data();
                let image_data: &ImageData = image_data.get_presenter_ref(frame).unwrap();

                let gfx_image = device
                    .build_image()
                    .extents(&image_data.size().cast::<u32>()[0..2])
                    .format(gfx::ImageFormat::SrgbRgba8)
                    .usage(flags![gfx::ImageUsage::{CopyWrite | Sampled}])
                    .build()?;
                let gfx_image = utils::UniqueImage::new(&**device, gfx_image);

                Ok(gfx_image)
            })
            .collect();
        let gfx_images = gfx_images?;

        // Allocate a heap to hold all those images
        fn to_res<'a>(x: &'a utils::UniqueImage<&gfx::Device>) -> gfx::ResourceRef<'a> {
            (&**x).into()
        }
        let allocs = self.image_heap.bind_multi(gfx_images.iter().map(to_res))?;

        // Store the allocated image (view) objects
        for ((&image_ptr, alloc), gfx_image) in self.new_images_list
            .iter()
            .zip(allocs.into_iter())
            .zip(gfx_images.into_iter())
        {
            let ref mut image: Image = images[image_ptr];

            let gfx_image_view = self.device
                .new_image_view(&*gfx_image, gfx::ImageLayout::ShaderRead)?;
            let gfx_image_view = utils::UniqueImageView::new(&*self.device, gfx_image_view);

            image.resident = Some(ResidentImageData {
                image: gfx_image.into_inner().1,
                image_view: gfx_image_view.into_inner().1,
                alloc,
                session_id: 0, // set later
            });
        }

        // Initiate the upload
        use std::cell::RefCell;
        let barrier_builder = RefCell::new(None);
        struct UploadRequest<'a> {
            device: &'a gfx::Device,
            frame: &'a PresenterFrame,
            image: &'a Image,
            barrier_builder: &'a RefCell<Option<Box<gfx::BarrierBuilder>>>,
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
                staging_buffer: &gfx::Buffer,
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
                    gfx::ImageLayout::CopyWrite,
                    gfx::ImageAspect::Color,
                    &gfx::ImageLayerRange {
                        mip_level: 0,
                        layers: 0..1,
                    },
                    &[0, 0],
                    &size.cast::<u32>()[0..2],
                );

                // Insert a image layout transition for all images in this
                // upload session
                let mut builder = self.barrier_builder.borrow_mut();
                if builder.is_none() {
                    *builder = Some(self.device.build_barrier());
                }
                let builder = builder.as_mut().unwrap();
                builder.image(
                    flags![gfx::AccessType::{CopyWrite}],
                    flags![gfx::AccessType::{FragmentRead}],
                    &resident.image,
                    gfx::ImageLayout::CopyWrite,
                    gfx::ImageLayout::ShaderRead,
                    &Default::default(),
                );

                Ok(())
            }

            fn post_copy(
                &self,
                encoder: &mut gfx::CopyCmdEncoder,
                _: &gfx::Buffer,
                _: Range<gfx::DeviceSize>,
            ) -> Result<()> {
                // For each upload session, this method is called after `copy`
                // was called for all images in the same session.
                // `self.barrier_builder` contains all image barriers for all
                // those images.
                let mut builder = self.barrier_builder.borrow_mut();
                if let Some(mut builder) = builder.take() {
                    let barrier: gfx::Barrier = builder.build()?;
                    encoder.barrier(&barrier);
                }
                Ok(())
            }
        }
        let session_id = self.uploader
            .upload(self.new_images_list.iter().map(&|&image_ptr| {
                let ref image: Image = images[image_ptr];
                UploadRequest {
                    device: &**device,
                    frame,
                    image,
                    barrier_builder: &barrier_builder,
                }
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
                uncommit_image(&**device, &mut self.image_heap, &resident)?;
            }
        }

        Ok(())
    }

    pub fn get(&self, image_ref: &ImageRef) -> Option<ResidentImage> {
        self.image_map.get(image_ref).map(|&i| ResidentImage {
            data: self.images[i]
                .resident
                .as_ref()
                .expect("The image is not queued for uploading yet"),
        })
    }

    pub fn get_fence_for_session(&self, session_id: uploader::SessionId) -> Option<&gfx::Fence> {
        self.uploader.get_fence(session_id)
    }
}

impl<'a> ResidentImage<'a> {
    pub fn image(&self) -> &gfx::Image {
        &self.data.image
    }

    pub fn image_view(&self) -> &gfx::ImageView {
        &self.data.image_view
    }

    pub fn session_id(&self) -> uploader::SessionId {
        self.data.session_id
    }
}
