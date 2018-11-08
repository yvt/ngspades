//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! The static data loder that supplies resources pre-initialized with
//! static data.
pub mod di {
    use injector::{prelude::*, Container};
    use std::sync::Arc;

    use super::*;
    use crate::di::DeviceContainer;

    pub trait StaticDataDeviceContainerExt {
        fn get_static_buffer<T: StaticBufferSource>(
            &self,
            source: &T,
        ) -> Option<&Arc<StaticBuffer>>;
        fn get_static_buffer_or_build<T: StaticBufferSource>(
            &mut self,
            source: &T,
        ) -> &Arc<StaticBuffer>;
        fn register_static_buffer_default<T: StaticBufferSource>(&mut self);

        fn get_static_image<T: StaticImageSource>(&self, source: &T) -> Option<&Arc<StaticImage>>;
        fn get_static_image_or_build<T: StaticImageSource>(
            &mut self,
            source: &T,
        ) -> &Arc<StaticImage>;
        fn register_static_image_default<T: StaticImageSource>(&mut self);

        fn get_quad_vertices_or_build(&mut self) -> &Arc<StaticBuffer> {
            self.get_static_buffer_or_build(&QuadVertices)
        }

        fn register_static_data_default(&mut self) {
            self.register_static_buffer_default::<QuadVertices>();
        }
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
    struct StaticBufferKey<T>(T);

    impl<T: StaticBufferSource> injector::Key for StaticBufferKey<T> {
        type Value = Arc<StaticBuffer>;
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
    struct StaticImageKey<T>(T);

    impl<T: StaticImageSource> injector::Key for StaticImageKey<T> {
        type Value = Arc<StaticImage>;
    }

    impl StaticDataDeviceContainerExt for Container {
        fn get_static_buffer<T: StaticBufferSource>(
            &self,
            source: &T,
        ) -> Option<&Arc<StaticBuffer>> {
            self.get(&StaticBufferKey(source.clone()))
        }

        fn get_static_buffer_or_build<T: StaticBufferSource>(
            &mut self,
            source: &T,
        ) -> &Arc<StaticBuffer> {
            self.get_or_build(&StaticBufferKey(source.clone())).unwrap()
        }

        fn register_static_buffer_default<T: StaticBufferSource>(&mut self) {
            self.register_factory(|key: &StaticBufferKey<T>, container| {
                let device = container.get_device().clone();

                let (main_queue, _) = container.get_main_queue().clone();

                use crate::asyncuploader::di::AsyncUploaderDeviceContainerExt;
                let uploader = match container.get_async_uploader_or_build() {
                    Ok(uploader) => uploader,
                    Err(err) => {
                        return StaticBuffer::with_error(err.kind().into());
                    }
                };

                StaticBuffer::new(device, main_queue, uploader, key.0.clone())
            });
        }

        fn get_static_image<T: StaticImageSource>(&self, source: &T) -> Option<&Arc<StaticImage>> {
            self.get(&StaticImageKey(source.clone()))
        }

        fn get_static_image_or_build<T: StaticImageSource>(
            &mut self,
            source: &T,
        ) -> &Arc<StaticImage> {
            self.get_or_build(&StaticImageKey(source.clone())).unwrap()
        }

        fn register_static_image_default<T: StaticImageSource>(&mut self) {
            self.register_factory(|key: &StaticImageKey<T>, container| {
                let device = container.get_device().clone();

                let (main_queue, _) = container.get_main_queue().clone();

                use crate::asyncuploader::di::AsyncUploaderDeviceContainerExt;
                let uploader = match container.get_async_uploader_or_build() {
                    Ok(uploader) => uploader,
                    Err(err) => {
                        return StaticImage::with_error(err.kind().into());
                    }
                };

                StaticImage::new(device, main_queue, uploader, key.0.clone())
            });
        }
    }
}

use arrayvec::ArrayVec;
use futures::{executor, future, future::Either, prelude::*, stream};
use ngsenumflags::flags;
use std::sync::{Arc, Mutex};
use zangfx::{
    base as gfx,
    // FIXME: `zangfx::common` is not meant to be used by an external client
    common::{FreezableCell, FreezableCellRef},
    prelude::*,
    utils::streamer::{StageBuffer, StageImage},
};

use crate::asyncuploader::{AsyncUploader, Request, UploadError};

#[derive(Debug)]
pub struct StaticData<T> {
    object_cell: Mutex<Option<T>>,
    complete_cell: FreezableCell<Option<gfx::Result<T>>>,
    join_handle_cell: Mutex<Option<executor::JoinHandle<(), Never>>>,
}

impl<T: Send + Sync + 'static> StaticData<T> {
    fn with_initiator<S, R>(
        uploader: &Arc<AsyncUploader>,
        initiator: impl FnOnce() -> gfx::Result<(T, S)> + Send + Sync + 'static,
    ) -> Arc<Self>
    where
        S: Stream<Item = R, Error = Never> + 'static,
        R: Request + 'static,
    {
        let this = Arc::new(Self {
            object_cell: Mutex::new(None),
            complete_cell: FreezableCell::new_unfrozen(None),
            join_handle_cell: Mutex::new(None),
        });

        // A `FnOnce() -> impl Stream` that produces zero or one requests.
        let source = {
            let this = Arc::clone(&this);

            move || {
                match initiator() {
                    Ok((object, requests)) => {
                        // A resource was created and is ready. Produce a stream
                        // containing an upload request for this buffer.
                        *this.object_cell.lock().unwrap() = Some(object);

                        Either::Left(requests)
                    }
                    Err(err) => {
                        // An error occured while creating and allocating a
                        // resource object. Report the error and return an empty
                        // stream.
                        let mut complete_cell_lock =
                            this.complete_cell.unfrozen_borrow_mut().unwrap();
                        *complete_cell_lock = Some(Err(err));
                        FreezableCellRef::freeze(complete_cell_lock);

                        Either::Right(stream::empty())
                    }
                }
            }
        };

        // Create a `Future` for uploading the buffer contents
        let future_upload = uploader.upload(source);

        let future = {
            let this = Arc::clone(&this);

            future_upload.then(move |result| {
                // Upload is complete. Store the result.
                let mut complete_cell_lock = this.complete_cell.unfrozen_borrow_mut().unwrap();

                match result {
                    Ok(()) => {
                        let buffer = this.object_cell.lock().unwrap().take().unwrap();
                        *complete_cell_lock = Some(Ok(buffer));
                    }
                    Err(UploadError::Device(err)) => {
                        *complete_cell_lock = Some(Err(err));
                    }
                    Err(UploadError::Cancelled) => {
                        // *shrug*
                    }
                }

                FreezableCellRef::freeze(complete_cell_lock);

                future::ok::<(), Never>(())
            })
        };

        // TODO: queue family ownership acquire operation

        // Initiate the upload
        let join_handle = executor::block_on(executor::spawn_with_handle(future)).unwrap();
        *this.join_handle_cell.lock().unwrap() = Some(join_handle);

        this
    }

    fn with_error(error: gfx::Error) -> Arc<Self> {
        Arc::new(Self {
            object_cell: Mutex::new(None),
            complete_cell: FreezableCell::new_frozen(Some(Err(error))),
            join_handle_cell: Mutex::new(None),
        })
    }

    fn resource(&self) -> Option<&gfx::Result<T>> {
        match self.complete_cell.frozen_borrow() {
            Ok(&Some(ref result)) => Some(result),
            _ => None,
        }
    }
}

impl<T> Drop for StaticData<T> {
    fn drop(&mut self) {
        if let Some(join_handle) = self.join_handle_cell.lock().unwrap().take() {
            executor::block_on(join_handle).unwrap();
        }
    }
}

pub type StaticBuffer = StaticData<gfx::BufferRef>;

pub trait StaticBufferSource:
    std::any::Any + Send + Sync + std::hash::Hash + std::cmp::Eq + Clone + std::fmt::Debug
{
    fn usage(&self) -> gfx::BufferUsageFlags {
        flags![gfx::BufferUsage::{CopyWrite | Uniform}]
    }

    fn bytes(&self) -> &[u8];
}

#[derive(Debug)]
struct BufferSourceToBytes<T>(T);

impl<T: StaticBufferSource> std::borrow::Borrow<[u8]> for BufferSourceToBytes<T> {
    fn borrow(&self) -> &[u8] {
        self.0.bytes()
    }
}

impl StaticBuffer {
    fn new(
        device: gfx::DeviceRef,
        queue: gfx::CmdQueueRef,
        uploader: &Arc<AsyncUploader>,
        source: impl StaticBufferSource,
    ) -> Arc<Self> {
        let uploader_2 = Arc::clone(uploader);

        let initiator = move || {
            // Create and allocate a buffer
            let buffer = device
                .build_buffer()
                .queue(&queue)
                .size(source.bytes().len() as _)
                .usage(source.usage())
                .build()?;

            let memory_type = device
                .try_choose_memory_type(
                    &buffer,
                    flags![gfx::MemoryTypeCaps::{DeviceLocal}],
                    flags![gfx::MemoryTypeCaps::{}],
                )?
                .unwrap();

            if !device.global_heap(memory_type).bind((&buffer).into())? {
                // Memory allocation failure of a static resource is fatal
                return Err(gfx::ErrorKind::OutOfDeviceMemory.into());
            }

            // Produce an upload request for this buffer.
            let buffer_proxy = uploader_2.make_buffer_proxy_if_needed(&buffer);
            let request = StageBuffer::new(buffer_proxy, 0, BufferSourceToBytes(source));

            let future_request = future::ok(request);
            let stream_request = stream::once(future_request);

            Ok((buffer, stream_request))
        };

        Self::with_initiator(uploader, initiator)
    }

    pub fn buffer(&self) -> Option<&gfx::Result<gfx::BufferRef>> {
        self.resource()
    }
}

pub type StaticImage = StaticData<gfx::ImageRef>;

pub unsafe trait StaticImageSource:
    std::any::Any + Send + Sync + std::hash::Hash + std::cmp::Eq + Clone + std::fmt::Debug
{
    fn usage(&self) -> gfx::ImageUsageFlags {
        flags![gfx::ImageUsage::{CopyWrite | Sampled}]
    }

    fn extents(&self) -> ImageExtents;

    fn format(&self) -> gfx::ImageFormat;

    /// The image data. Must be large enough to contain entire the image.
    /// (This is why this trait is marked as `unsafe`.)
    fn bytes(&self) -> &[u8];
}

#[derive(Debug, Clone)]
pub enum ImageExtents {
    Normal(ArrayVec<[u32; 3]>),
    Cube(u32),
}

trait ImageBuilderExt: gfx::ImageBuilder {
    fn image_extents(&mut self, v: &ImageExtents) -> &mut dyn gfx::ImageBuilder {
        match v {
            ImageExtents::Normal(x) => self.extents(&x),
            ImageExtents::Cube(x) => self.extents_cube(*x),
        }
    }
}

impl ImageBuilderExt for dyn gfx::ImageBuilder {}

#[derive(Debug)]
struct ImageSourceToBytes<T>(T);

impl<T: StaticImageSource> std::borrow::Borrow<[u8]> for ImageSourceToBytes<T> {
    fn borrow(&self) -> &[u8] {
        self.0.bytes()
    }
}

impl StaticImage {
    fn new(
        device: gfx::DeviceRef,
        queue: gfx::CmdQueueRef,
        uploader: &Arc<AsyncUploader>,
        source: impl StaticImageSource,
    ) -> Arc<Self> {
        let uploader_2 = Arc::clone(uploader);

        let initiator = move || {
            // Create and allocate a image
            let image = device
                .build_image()
                .queue(&queue)
                .image_extents(&source.extents())
                .usage(source.usage())
                .format(source.format())
                .build()?;

            let memory_type = device
                .try_choose_memory_type(
                    &image,
                    flags![gfx::MemoryTypeCaps::{DeviceLocal}],
                    flags![gfx::MemoryTypeCaps::{}],
                )?
                .unwrap();

            if !device.global_heap(memory_type).bind((&image).into())? {
                // Memory allocation failure of a static resource is fatal
                return Err(gfx::ErrorKind::OutOfDeviceMemory.into());
            }

            // Produce an upload request for this image.
            let size = match source.extents() {
                ImageExtents::Normal(x) => x.clone(),
                ImageExtents::Cube(_) => unimplemented!(),
            };
            let image_proxy = uploader_2.make_image_proxy_if_needed(&image);
            let request = StageImage::new_default(image_proxy, ImageSourceToBytes(source), &size);

            let future_request = future::ok(request);
            let stream_request = stream::once(future_request);

            Ok((image, stream_request))
        };

        Self::with_initiator(uploader, initiator)
    }

    pub fn image(&self) -> Option<&gfx::Result<gfx::ImageRef>> {
        self.resource()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct QuadVertices;

impl StaticBufferSource for QuadVertices {
    fn usage(&self) -> gfx::BufferUsageFlags {
        flags![gfx::BufferUsage::{CopyWrite | Vertex}]
    }

    fn bytes(&self) -> &[u8] {
        static VERTICES: &[[u16; 2]; 4] = &[[0, 0], [1, 0], [0, 1], [1, 1]];
        pod::Pod::as_bytes(VERTICES)
    }
}
