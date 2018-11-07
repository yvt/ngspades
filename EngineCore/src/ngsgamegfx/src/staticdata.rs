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
    }
}

use futures::{executor, future, future::Either, prelude::*, stream};
use ngsenumflags::flags;
use std::sync::{Arc, Mutex};
use zangfx::{
    base as gfx,
    // FIXME: `zangfx::common` is not meant to be used by an external client
    common::{FreezableCell, FreezableCellRef},
    prelude::*,
    utils::streamer::StageBuffer,
};

use crate::asyncuploader::{AsyncUploader, UploadError};

pub trait StaticBufferSource:
    std::any::Any + Send + Sync + std::hash::Hash + std::cmp::Eq + Clone + std::fmt::Debug
{
    fn usage(&self) -> gfx::BufferUsageFlags {
        flags![gfx::BufferUsage::{CopyWrite | Uniform}]
    }

    fn bytes(&self) -> &[u8];
}

#[derive(Debug)]
pub struct StaticBuffer {
    buffer_cell: Mutex<Option<gfx::BufferRef>>,
    complete_cell: FreezableCell<Option<gfx::Result<gfx::BufferRef>>>,
    join_handle_cell: Mutex<Option<executor::JoinHandle<(), Never>>>,
}

#[derive(Debug)]
struct BufferSourceToBytes<T>(T);

impl<T: StaticBufferSource> std::borrow::Borrow<[u8]> for BufferSourceToBytes<T> {
    fn borrow(&self) -> &[u8] {
        self.0.bytes()
    }
}

impl StaticBuffer {
    fn new<T: StaticBufferSource>(
        device: gfx::DeviceRef,
        queue: gfx::CmdQueueRef,
        uploader: &Arc<AsyncUploader>,
        source: T,
    ) -> Arc<Self> {
        let this = Arc::new(Self {
            buffer_cell: Mutex::new(None),
            complete_cell: FreezableCell::new_unfrozen(None),
            join_handle_cell: Mutex::new(None),
        });

        // A `FnOnce() -> impl Stream` that produces zero or one requests.
        let source = {
            let this = Arc::clone(&this);
            let uploader = Arc::clone(uploader);

            move || {
                match (|| {
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

                    Ok(buffer)
                })() {
                    Ok(buffer) => {
                        // A buffer was created and is ready. Produce a stream
                        // containing an upload request for this buffer.
                        let buffer_proxy = uploader.make_buffer_proxy_if_needed(&buffer);

                        *this.buffer_cell.lock().unwrap() = Some(buffer);

                        let request =
                            StageBuffer::new(buffer_proxy, 0, BufferSourceToBytes(source));
                        let future_request = future::ok(request);
                        Either::Left(stream::once(future_request))
                    }
                    Err(e) => {
                        // An error occured while creating and allocating a
                        // buffer. Report the error and return an empty stream.
                        let mut complete_cell_lock =
                            this.complete_cell.unfrozen_borrow_mut().unwrap();
                        *complete_cell_lock = Some(Err(e));
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
                        let buffer = this.buffer_cell.lock().unwrap().take().unwrap();
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
            buffer_cell: Mutex::new(None),
            complete_cell: FreezableCell::new_frozen(Some(Err(error))),
            join_handle_cell: Mutex::new(None),
        })
    }

    pub fn buffer(&self) -> Option<&gfx::Result<gfx::BufferRef>> {
        match self.complete_cell.frozen_borrow() {
            Ok(&Some(ref result)) => Some(result),
            _ => None,
        }
    }
}

impl Drop for StaticBuffer {
    fn drop(&mut self) {
        if let Some(join_handle) = self.join_handle_cell.lock().unwrap().take() {
            executor::block_on(join_handle).unwrap();
        }
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
