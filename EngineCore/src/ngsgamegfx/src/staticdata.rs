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
    use crate::{di::DeviceContainer, spawner::di::SpawnerDeviceContainerExt};

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

        /// Return a reference to `StaticBuffer` of a buffer containing
        /// `[[u16; 2]; 4]` that represents the vertices of the rectangle
        /// `x, y ∈ [0, 1]`, sorted in the triangle strip order.
        fn get_quad_vertices_or_build(&mut self) -> &Arc<StaticBuffer> {
            self.get_static_buffer_or_build(&QuadVertices)
        }

        /// Return a reference to `StaticBuffer` of a buffer containing
        /// `[[u16; 2]; 3]` that represents the vertices of a large triangle
        /// covering the rectangle `x, y ∈ [0, 1]`.
        fn get_huge_triangle_vertices_or_build(&mut self) -> &Arc<StaticBuffer> {
            self.get_static_buffer_or_build(&HugeTriangleVertices)
        }

        /// Return a reference to `StaticImage` of a 2D RGBA image containing
        /// a single white pixel.
        fn get_white_image_or_build(&mut self) -> &Arc<StaticImage> {
            self.get_static_image_or_build(&WhiteImage)
        }

        /// Return a reference to `StaticImage` of a 256x256 2D RGBA image
        /// containing random data.
        fn get_noise_image_or_build(&mut self) -> &Arc<StaticImage> {
            self.get_static_image_or_build(&NoiseImage)
        }

        fn register_static_data_default(&mut self) {
            self.register_static_buffer_default::<QuadVertices>();
            self.register_static_buffer_default::<HugeTriangleVertices>();
            self.register_static_image_default::<WhiteImage>();
            self.register_static_image_default::<NoiseImage>();
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

                let spawner = container.get_spawner_or_build().clone();

                use crate::asyncuploader::di::AsyncUploaderDeviceContainerExt;
                let uploader = match container.get_async_uploader_or_build() {
                    Ok(uploader) => uploader,
                    Err(err) => {
                        return StaticBuffer::with_error(err.kind().into());
                    }
                };

                StaticBuffer::new(device, main_queue, uploader, &spawner, key.0.clone())
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

                let spawner = container.get_spawner_or_build().clone();

                use crate::asyncuploader::di::AsyncUploaderDeviceContainerExt;
                let uploader = match container.get_async_uploader_or_build() {
                    Ok(uploader) => uploader,
                    Err(err) => {
                        return StaticImage::with_error(err.kind().into());
                    }
                };

                StaticImage::new(device, main_queue, uploader, &spawner, key.0.clone())
            });
        }
    }
}

use arrayvec::ArrayVec;
use asynclazy::Async;
use either::Either;
use flags_macro::flags;
use futures::{future, prelude::*, stream};
use std::{
    marker::Unpin,
    sync::{Arc, Mutex},
};
use zangfx::{
    base as gfx,
    prelude::*,
    utils::streamer::{StageBuffer, StageImage},
};

use crate::{
    asyncuploader::{AsyncUploader, Request, UploadError},
    spawner::Spawner,
};

#[derive(Debug)]
pub struct StaticData<T> {
    object_cell: Async<Option<gfx::Result<T>>>,
}

impl<T: Send + Sync + 'static> StaticData<T> {
    fn with_initiator<S, R>(
        uploader: &Arc<AsyncUploader>,
        spawner: &Arc<dyn Spawner>,
        initiator: impl FnOnce() -> gfx::Result<(T, S)> + Send + Sync + 'static,
    ) -> Arc<Self>
    where
        S: Stream<Item = R> + Unpin + 'static,
        R: Request + Unpin + 'static,
    {
        let mut spawner = spawner.get_spawn();

        let object_cell = Arc::new(Mutex::new(None));

        // A `FnOnce() -> impl Stream` that produces zero or one requests.
        let source = {
            let object_cell = Arc::clone(&object_cell);

            move || {
                match initiator() {
                    Ok((object, requests)) => {
                        // A resource was created and is ready. Store the
                        // created object in `object_cell` temporarily so that
                        // it can be retrieved again when upload is complete.
                        *object_cell.lock().unwrap() = Some(Ok(object));

                        // Produce a stream containing an upload request for
                        // this buffer.
                        Either::Left(requests)
                    }
                    Err(err) => {
                        // An error occured while creating and allocating a
                        // resource object. Report the error.
                        *object_cell.lock().unwrap() = Some(Err(err));

                        // Return an empty stream.
                        Either::Right(stream::empty())
                    }
                }
            }
        };

        // Initiate upload. This returns a `Future` that becomes ready when
        // upload is complete.
        let future_upload = uploader.upload(source);

        // And then for storing the result...
        let future = future_upload.map(move |result| {
            let object_cell = object_cell.lock().unwrap().take().unwrap();

            match object_cell {
                Ok(obj) => {
                    // Upload is complete. Store the result.
                    match result {
                        Ok(()) => Some(Ok(obj)),
                        Err(UploadError::Device(err)) => Some(Err(err)),
                        Err(UploadError::Cancelled) => None,
                    }
                }

                Err(err) => {
                    // Upload did not take place because of an error in the
                    // first stage.
                    Some(Err(err))
                }
            }
        });

        // TODO: queue family ownership acquire operation

        // Create a cell for receiving the result
        let object_cell = Async::with_future(&mut *spawner, future).unwrap();

        Arc::new(Self { object_cell })
    }

    fn with_error(error: gfx::Error) -> Arc<Self> {
        Arc::new(Self {
            object_cell: Async::with_value(Some(Err(error))),
        })
    }

    fn resource(&self) -> Option<&gfx::Result<T>> {
        match self.object_cell.try_get() {
            Some(Some(result)) => Some(result),

            // Upload was cancelled because the uploader was torn down
            Some(None) => None,

            // The result is not ready yet
            None => None,
        }
    }
}

impl<T> Drop for StaticData<T> {
    fn drop(&mut self) {
        // Block until the computation is done
        self.object_cell.get();
    }
}

pub type StaticBuffer = StaticData<gfx::BufferRef>;

pub trait StaticBufferSource:
    std::any::Any + Send + Sync + std::hash::Hash + std::cmp::Eq + Clone + std::fmt::Debug + Unpin
{
    fn usage(&self) -> gfx::BufferUsageFlags {
        flags![gfx::BufferUsageFlags::{CopyWrite | Uniform}]
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
        spawner: &Arc<dyn Spawner>,
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
                    flags![gfx::MemoryTypeCapsFlags::{DeviceLocal}],
                    flags![gfx::MemoryTypeCapsFlags::{}],
                )?
                .unwrap();

            if !device.global_heap(memory_type).bind((&buffer).into())? {
                // Memory allocation failure of a static resource is fatal
                return Err(gfx::ErrorKind::OutOfDeviceMemory.into());
            }

            // Produce an upload request for this buffer.
            let buffer_proxy = uploader_2.make_buffer_proxy_if_needed(&buffer);
            let request = StageBuffer::new(buffer_proxy, 0, BufferSourceToBytes(source));

            let future_request = future::ready(request);
            let stream_request = stream::once(future_request);

            Ok((buffer, stream_request))
        };

        Self::with_initiator(uploader, spawner, initiator)
    }

    pub fn buffer(&self) -> Option<&gfx::Result<gfx::BufferRef>> {
        self.resource()
    }
}

pub type StaticImage = StaticData<gfx::ImageRef>;

pub unsafe trait StaticImageSource:
    std::any::Any + Send + Sync + std::hash::Hash + std::cmp::Eq + Clone + std::fmt::Debug + Unpin
{
    fn usage(&self) -> gfx::ImageUsageFlags {
        flags![gfx::ImageUsageFlags::{CopyWrite | Sampled}]
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
        spawner: &Arc<dyn Spawner>,
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
                    flags![gfx::MemoryTypeCapsFlags::{DeviceLocal}],
                    flags![gfx::MemoryTypeCapsFlags::{}],
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

            let future_request = future::ready(request);
            let stream_request = stream::once(future_request);

            Ok((image, stream_request))
        };

        Self::with_initiator(uploader, spawner, initiator)
    }

    pub fn image(&self) -> Option<&gfx::Result<gfx::ImageRef>> {
        self.resource()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct QuadVertices;

impl StaticBufferSource for QuadVertices {
    fn usage(&self) -> gfx::BufferUsageFlags {
        flags![gfx::BufferUsageFlags::{CopyWrite | Vertex}]
    }

    fn bytes(&self) -> &[u8] {
        static VERTICES: &[[u16; 2]; 4] = &[[0, 0], [1, 0], [0, 1], [1, 1]];
        pod::Pod::as_bytes(VERTICES)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct HugeTriangleVertices;

impl StaticBufferSource for HugeTriangleVertices {
    fn usage(&self) -> gfx::BufferUsageFlags {
        flags![gfx::BufferUsageFlags::{CopyWrite | Vertex}]
    }

    fn bytes(&self) -> &[u8] {
        static VERTICES: &[[u16; 2]; 3] = &[[0, 0], [2, 0], [0, 2]];
        pod::Pod::as_bytes(VERTICES)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct WhiteImage;

unsafe impl StaticImageSource for WhiteImage {
    fn extents(&self) -> ImageExtents {
        ImageExtents::Normal([1, 1].iter().cloned().collect())
    }

    fn format(&self) -> gfx::ImageFormat {
        <u8>::as_rgba_norm()
    }

    fn bytes(&self) -> &[u8] {
        &[0xff; 4]
    }
}

use lazy_static::lazy_static;
use rand::{thread_rng, Rng};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
struct NoiseImage;

unsafe impl StaticImageSource for NoiseImage {
    fn extents(&self) -> ImageExtents {
        ImageExtents::Normal([256, 256].iter().cloned().collect())
    }

    fn format(&self) -> gfx::ImageFormat {
        <u8>::as_rgba_norm()
    }

    fn bytes(&self) -> &[u8] {
        lazy_static! {
            static ref BYTES: Vec<u8> = {
                let mut bytes = vec![0; 256 * 256 * 4];
                thread_rng().fill(&mut bytes[..]);
                bytes
            };
        }
        BYTES.as_slice()
    }
}
