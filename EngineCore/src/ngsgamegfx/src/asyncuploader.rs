//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Facilitates asynchronous staging operations through a `Future`-based
//! interface.
pub mod di {
    use injector::{prelude::*, Container};
    use std::sync::Arc;

    use super::*;
    use crate::di::DeviceContainer;

    pub trait DeviceContainerExt {
        fn get_async_uploader(&self) -> Option<&gfx::Result<Arc<AsyncUploader>>>;
        fn get_async_uploader_or_build(&mut self) -> &gfx::Result<Arc<AsyncUploader>>;
        fn register_async_uploader_default(&mut self);
    }

    impl DeviceContainerExt for Container {
        fn get_async_uploader(&self) -> Option<&gfx::Result<Arc<AsyncUploader>>> {
            self.get_singleton()
        }

        fn get_async_uploader_or_build(&mut self) -> &gfx::Result<Arc<AsyncUploader>> {
            self.get_singleton_or_build().unwrap()
        }

        fn register_async_uploader_default(&mut self) {
            self.register_singleton_factory(|container| {
                let device = container.get_device().clone();
                let (main_queue, main_queue_family) = container.get_main_queue().clone();

                let queue;
                let queue_ownership_transfer;

                // Is a copy queue available?
                if let Some((copy_queue, copy_queue_family)) = container.get_copy_queue() {
                    // Yep :)
                    queue = copy_queue.clone();
                    if *copy_queue_family == main_queue_family {
                        queue_ownership_transfer = None;
                    } else {
                        queue_ownership_transfer = Some([*copy_queue_family, main_queue_family]);
                    }
                } else {
                    // Nope :(
                    queue = main_queue.clone();
                    queue_ownership_transfer = None;
                }

                AsyncUploader::new(device, queue, queue_ownership_transfer).map(Arc::new)
            });
        }
    }
}

use boxfnonce::SendBoxFnOnce;
use futures::{
    channel::{mpsc, oneshot},
    executor,
    prelude::*,
    Future, Stream,
};
use std::{
    fmt,
    ops::Range,
    sync::{Arc, Mutex},
    thread,
};

use zangfx::{base as gfx, utils::streamer};

/// Facilitates asynchronous staging operations through a `Future`-based
/// interface.
///
/// `AsyncUploader` accepts requests via the `upload` method, which takes a
/// function that is executed, returning a stream of actual requests
/// (`impl Stream<Item = impl CopyRequest + Debug, Error = Never>`).
/// The processing of requests takes place entirely in a dedicated background
/// thread.
///
/// Requests generate GPU commands which are submitted to the copy queue
/// (`DeviceContainer::get_copy_queue()`) if possible. This means that if
/// `get_copy_queue()` is `None`:
///
///  - Copy commands must operate on proxy objects created via `make_proxy`
///    for the copy queue.
///  - Queue family ownership transfer operations must be manually inserted if
///    the copy queue belongs to a different queue family from one where the
///    staged resourecs are consumed.
///
/// TODO: Support the cases where `get_copy_queue()` is not `None`
pub struct AsyncUploader {
    shared: Arc<Shared>,
    sender: mpsc::UnboundedSender<ChannelPayload>,
    queue_ownership_transfer: Option<[gfx::QueueFamily; 2]>,
    join_handle: Option<thread::JoinHandle<()>>,
}

type ChannelPayload = SendBoxFnOnce<'static, (), StreamerRequestStream>;

type StreamerRequestStream = Box<dyn Stream<Item = StreamerRequest, Error = Never>>;

#[derive(Debug)]
pub enum UploadError {
    Cancelled,
    Device(gfx::Error),
}

#[derive(Debug)]
struct Shared {
    /// If the streamer fails, the error will be stored here.
    error: Mutex<Option<gfx::ErrorKind>>,
}

impl AsyncUploader {
    fn new(
        device: gfx::DeviceRef,
        queue: gfx::CmdQueueRef,
        queue_ownership_transfer: Option<[gfx::QueueFamily; 2]>,
    ) -> gfx::Result<Self> {
        let (sender, receiver) = mpsc::unbounded();

        let shared = Arc::new(Shared {
            error: Mutex::new(None),
        });

        let join_handle = {
            let shared = Arc::clone(&shared);

            thread::Builder::new()
                .name("AsyncUploader".into())
                .spawn(move || {
                    if let Err(err) = (|| {
                        let mut cmd_generator = streamer::CopyCmdGenerator::new();

                        if let Some([_, dst_queue_family]) = queue_ownership_transfer {
                            // Perform ownership release operations after staging
                            cmd_generator.dst_queue_family = Some(dst_queue_family);
                        }

                        let streamer = streamer::Builder::default(device, queue)
                            .with_cmd_generator(cmd_generator)
                            .with_batch_size(1024 * 1024 * 10)
                            .build_with_heap_size(1024 * 1024 * 100)?;

                        let request_stream = receiver
                            .map(|x: ChannelPayload| x.call())
                            .flatten()
                            .map_err(|_: Never| -> gfx::Error { unreachable!() });

                        let result = streamer.send_all(request_stream);

                        let mut pool = executor::LocalPool::new();

                        pool.run_until(result, &mut pool.executor())
                    })() {
                        // Something went wrong in the uploader thread.
                        // Store the error reason before hanging up the receiver.
                        *shared.error.lock().unwrap() = Some(err.kind());
                    }
                })
                .expect("Failed to start an uploader thread.")
        };

        Ok(Self {
            shared,
            sender,
            queue_ownership_transfer,
            join_handle: Some(join_handle),
        })
    }

    /// Indicates a queue family ownership transfer operation required between
    /// the upload and use of resources.
    ///
    /// If the returned value is `Some([a, b])`, the clients must insert a
    /// ownership transfer operation from the queue family `a` to `b`. Each
    /// indicates the copy and main queue family, respectively.
    ///
    /// They don't have to if the returned value is `None`.
    pub fn queue_ownership_transfer_families(&self) -> Option<[gfx::QueueFamily; 2]> {
        self.queue_ownership_transfer
    }

    /// Initiate upload requests.
    ///
    /// A supplied function is called when the uploader is ready to accept new
    /// requests. The stream returned by the function can be non-`Send`.
    ///
    /// The returned `Future` completes when all requests generated by the
    /// stream are completed, i.e., all command buffers involved with the
    /// requests have completed execution.
    pub fn upload<T, R>(
        &self,
        request_source: impl 'static + Send + Sync + FnOnce() -> T,
    ) -> impl Future<Item = (), Error = UploadError> + Send + Sync + 'static
    where
        T: Stream<Item = R, Error = Never> + 'static,
        R: Request + 'static,
    {
        // Use this channel to notify the completion
        let (sender, receiver) = oneshot::channel();

        let ref shared = self.shared;

        // This `Send`-able closure is executed on the uploader thread and
        // returns a non-`Send`-able `Stream`
        let payload = move || {
            use crate::utils::futures::PrivateStreamExt;

            let sender_cell = Some(sender);

            let stream = request_source().with_terminator().map_with_state(
                sender_cell,
                |(req, is_last), sender_cell| {
                    let sender = if is_last {
                        // Notify the completion
                        debug_assert!(sender_cell.is_some());
                        sender_cell.take()
                    } else {
                        None
                    };
                    StreamerRequest(Box::new(req), sender)
                },
            );

            Box::new(stream) as StreamerRequestStream
        };

        // Submission fails if the uploader thread is already down. In that
        // case, we'll know it via `receiver` returning `Err(Canceled)`.
        let _ = self.sender.unbounded_send(payload.into());

        let shared = Arc::clone(shared);
        receiver.map_err(move |_| {
            // `sender` was dropped before the result is sent back. This
            // indicates that the uploader thread died for some reasons.
            //
            // `Shared::result()` gives us a clue about the cause of the death.
            shared.result()
        })
    }
}

impl Drop for AsyncUploader {
    fn drop(&mut self) {
        // FIXME: Cancel all pending requests?
        self.sender.close_channel();
        self.join_handle.take().unwrap().join().unwrap();
    }
}

impl fmt::Debug for AsyncUploader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AsyncUploader")
            .field("shared", &self.shared)
            .field("join_handle", &self.join_handle)
            .field("sender", &())
            .field("queue_ownership_transfer", &self.queue_ownership_transfer)
            .finish()
    }
}

impl Shared {
    /// If called after the uploader thread exited, returns an `UploadError`
    /// explaining the reason.
    fn result(&self) -> UploadError {
        if let Some(error_kind) = *self.error.lock().unwrap() {
            UploadError::Device(gfx::Error::new(error_kind))
        } else {
            UploadError::Cancelled
        }
    }
}

/// An upload request consumed by `AsyncUploader`.
///
/// Note the lack of `Send` and `Sync` in its trait bounds.
pub trait Request: streamer::CopyRequest + fmt::Debug {}
impl<T: ?Sized + streamer::CopyRequest + fmt::Debug> Request for T {}

/// Type-erasing container of `Request` that implements
/// `zangfx::utils::streamer::StreamerRequest`.
#[derive(Debug)]
struct StreamerRequest(Box<dyn Request + 'static>, Option<oneshot::Sender<()>>);

impl streamer::Request for StreamerRequest {
    fn size(&self) -> usize {
        self.0.size()
    }

    fn populate(&mut self, staging_buffer: &mut [u8]) {
        self.0.populate(staging_buffer);
    }

    fn exfiltrate(&mut self, staging_buffer: &[volatile_view::Volatile<u8>]) {
        self.0.exfiltrate(staging_buffer);

        if let Some(x) = self.1.take() {
            let _ = x.send(()); // Ignore send failure
        }
    }
}

impl streamer::CopyRequest for StreamerRequest {
    fn copy(
        &mut self,
        encoder: &mut dyn gfx::CopyCmdEncoder,
        staging_buffer: &gfx::BufferRef,
        staging_buffer_range: Range<gfx::DeviceSize>,
    ) -> gfx::Result<()> {
        self.0.copy(encoder, staging_buffer, staging_buffer_range)
    }

    fn queue_ownership_acquire(&self) -> Option<gfx::QueueOwnershipTransfer<'_>> {
        self.0.queue_ownership_acquire()
    }

    fn queue_ownership_release(&self) -> Option<gfx::QueueOwnershipTransfer<'_>> {
        self.0.queue_ownership_release()
    }
}
