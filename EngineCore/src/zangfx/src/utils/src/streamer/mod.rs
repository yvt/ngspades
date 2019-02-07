//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Facilitates transfer operations between the host and device by providing
//! automatic allocation from a internally-managed fixed-size staging buffer
//! and coordination of command buffer generation and submission.
//!
//! <a href="https://derpibooru.org/868433?"
//! title="&quot;Oh no, that's too many balloons. More candy! Oh, less candy. Oh wait, I know. Streamers!&quot; — Pinkie Pie">
//! ![""](https://derpicdn.net/img/2015/4/8/868433/large.png)
//! </a>
//!
//! # Terminology
//!
//! - A *request* is a user-supplied object describing a transfer operation
//!   as well as the amount of staging buffer required during that operation.
//! - A *batch* is made of one or more requests and is associated with a single
//!   command buffer and a single consecutive region inside a staging buffer.
//! - A *command generator* converts requests into device commands.
//!
//! # Basics of the operation
//!
//! [`Streamer`] is a `Sink` accepting a stream of user-supplied
//! `impl `[`Request`]s each representing a *request*.
//!
//! The streaming operation is performed in a unit called a *batch* each
//! composed of one or more requests.
//! For each request, a portion of the staging buffer whose size is specified
//! via [`size`] is allocated, [`populate`] is called to initialize the
//! contents of the allocated portion, and the request is added to the current
//! batch.
//! This step is repeated until (1) the total amount of portions allocated for
//! the current batch reaches the maximum batch size specified via
//! [`Builder::batch_size`] or (2) there are no more requests to process.
//! At this point, the batch is said to be *sealed*.
//!
//! [`Streamer`]: crate::streamer::Streamer
//! [`Request`]: crate::streamer::Request
//! [`size`]: crate::streamer::Request::size
//! [`populate`]: crate::streamer::Request::populate
//! [`Builder::batch_size`]: crate::streamer::Builder::batch_size
//!
//! After a batch is sealed, a command buffer is constructed for that batch.
//! A [`CmdGenerator`] supplied as a part of `Builder` is responsible
//! for the coordination of command buffer generation. It receives a staging
//! buffer, requests, and their corresponding buffer ranges, but it's up to
//! `CmdGenerator` how commands are generated from them. For instance,
//! [`CopyCmdGenerator`] (which is the default choice) creates a single copy
//! command encoder, allowing requests implementing [`CopyRequest`] to encode
//! arbitrary copy commands.
//! After that, the command buffer is queued for execution. `CmdQueue` is
//! automatically flushed.
//!
//! [`CmdGenerator`]: crate::streamer::CmdGenerator
//! [`CopyCmdGenerator`]: crate::streamer::CopyCmdGenerator
//! [`CopyRequest`]: crate::streamer::CopyRequest
//!
//! Upon command buffer completion, [`exfiltrate`] is called to give requests a
//! chance to extract the data stored in the staging buffer (for device-to-host
//! transfer). Finally, the allocated portions of the staging buffer are
//! released.
//!
//! [`exfiltrate`]: crate::streamer::Request::exfiltrate
//!
//! # Error handling
//!
//! `Streamer` only handles fatal error conditions such as device loss.
//! Implementors of `Request` can only return the errors relevant to the
//! operation of `Streamer`, and should handle other kinds of errors through
//! other means.
//!
use futures::{prelude::*, task, try_ready, Future, Poll, Sink};
use std::{
    borrow::Borrow,
    collections::VecDeque,
    marker::Unpin,
    ops::Range,
    pin::Pin,
};
use volatile_view::Volatile;

use zangfx_base::{self as base, DeviceSize, Result};

use crate::{
    asyncheap::{AsyncHeap, Bind},
    buffer::BufferUtils,
    futuresapi::{CmdBufferFutureExt, CmdBufferResult},
};

mod utils;
pub use self::utils::*;

/// Supplies parameters for [`Streamer`].
#[derive(Debug, Clone)]
pub struct Builder<G> {
    pub device: base::DeviceRef,

    pub queue: base::CmdQueueRef,

    pub cmd_generator: G,

    /// The maximum number of bytes transferred per batch.
    pub batch_size: usize,

    /// Specifies whether flushing requires that all command buffers are
    /// completed.
    ///
    /// When set to `false`, flushing only ensures that the command buffer
    /// has been submitted. `Streamer` might not check the command buffer
    /// completion at all, which have some ramifications:
    ///
    /// - You won't be able to use `Request::exfiltrate`.
    /// - Necessitates uses of graphics API-level synchronization primitives
    ///   such as *semaphores* (for inter-queue synchronization) and *fences*
    ///   (for intra-queue synchronization).
    /// - `Streamer` won't release staging buffers after flushing, which
    ///   impades the usage of multiple `Streamer`s sharing a single
    ///   `AsyncHeap`.
    ///
    pub should_wait_completion: bool,
}

impl Builder<CopyCmdGenerator> {
    /// Consturct a `Builder` with supplied objects and default values
    /// for the other fields.
    ///
    /// This method uses `CopyCmdGenerator::new()` as the default command
    /// generator. Use [`with_cmd_generator`] to provide a custom one.
    ///
    /// [`with_cmd_generator`]: Builder::with_cmd_generator
    pub fn default(device: base::DeviceRef, queue: base::CmdQueueRef) -> Self {
        Self {
            device,
            queue,
            cmd_generator: CopyCmdGenerator::new(),
            batch_size: 1024 * 1024,
            should_wait_completion: true,
        }
    }
}

impl<G> Builder<G> {
    /// Return `self` with a new value for the `cmd_generator` field.
    pub fn with_cmd_generator<NG>(self, cmd_generator: NG) -> Builder<NG> {
        Builder {
            device: self.device,
            queue: self.queue,
            cmd_generator,
            batch_size: self.batch_size,
            should_wait_completion: self.should_wait_completion,
        }
    }

    /// Return `self` with a new value for the `batch_size` field.
    pub fn with_batch_size(self, batch_size: usize) -> Self {
        Self { batch_size, ..self }
    }

    /// Return `self` with a new value for the `should_wait_completion` field.
    pub fn with_should_wait_completion(self, should_wait_completion: bool) -> Self {
        Self {
            should_wait_completion,
            ..self
        }
    }

    /// Build a [`Streamer`], consuming `self` and a given `AsyncHeap`.
    pub fn build_with_heap<T, H>(self, heap: H) -> Streamer<T, H, G>
    where
        T: Unpin + Request,
        H: Unpin + Borrow<AsyncHeap>,
        G: Unpin + CmdGenerator<T>,
    {
        Streamer::new(self, heap)
    }

    /// Build a [`Streamer`], consuming `self`. A new `AsyncHeap` with a
    /// specified size, suitable for the `COPY_READ` usage is automatically
    /// constructed during the process.
    pub fn build_with_heap_size<T>(self, heap_size: DeviceSize) -> Result<Streamer<T, AsyncHeap, G>>
    where
        T: Unpin + Request,
        G: Unpin + CmdGenerator<T>,
    {
        use crate::prelude::*;

        let heap = self
            .device
            .build_dynamic_heap()
            .memory_type(
                self.device
                    .try_choose_memory_type_shared(base::BufferUsageFlags::COPY_READ)?
                    .unwrap(),
            )
            .size(heap_size)
            .build()?;

        Ok(self.build_with_heap(AsyncHeap::new(heap)))
    }
}

/// Generates device commands for a batch of request type `T`.
pub trait CmdGenerator<T> {
    /// Generate device commands for a batch and encode them into `cmd_buffer`.
    fn encode(
        &mut self,
        cmd_buffer: &mut base::CmdBufferRef,
        staging_buffer: &base::BufferRef,
        requests: &mut [(T, Range<DeviceSize>)],
    ) -> Result<()>;
}

/// A request to be processed by [`Streamer`].
pub trait Request {
    /// The number of bytes required in the staging buffer.
    fn size(&self) -> usize;

    /// Fill the staging buffer with the contents.
    fn populate(&mut self, _staging_buffer: &mut [u8]) {}

    /// Get the usage flags required for the staging buffer.
    fn staging_buffer_usage(&self) -> base::BufferUsageFlags {
        base::BufferUsageFlags::COPY_READ
    }

    /// Retrieve the data stored in the staging buffer after the device operation.
    ///
    /// `should_wait_completion` must be set to `true` (which is the default
    /// value) for this method to be called reliably.
    ///
    /// Note that you must issue a `CmdBuffer::host_barrier` command to ensure
    /// writes to the staging buffer are visible to the host. You'll get
    /// inconsistent data otherwise, which is why `Streamer` can't simply
    /// pass `&mut [u8]` here.
    fn exfiltrate(&mut self, _staging_buffer: &[Volatile<u8>]) {}
}

/// Facilitates transfer operations between the host and device by providing
/// automatic allocation from a internally-managed fixed-size staging buffer
/// and coordination of command buffer generation and submission.
///
/// See [the module-level documentation](index.html) for details.
//
/// # Type parameters
///
///  - `T: `[`Request`] - A type representing requests.
///  - `H: Borrow<`[`AsyncHeap`]`>` - `AsyncHeap` or something that can be used
///    to borrow a reference to `AsyncHeap`. A value of this type is supplied at
///    construction time. Staging buffers are allocated from that.
///
/// [`AsyncHeap`]: crate::asyncheap::AsyncHeap
///
#[derive(Debug)]
pub struct Streamer<T, H, G> {
    device: base::DeviceRef,
    queue: base::CmdQueueRef,
    should_wait_completion: bool,
    max_batch_size: DeviceSize,

    /// A single-sized queue at the input stage. We need this because we can't
    /// tell if we can insert a new request into `next_batch` without knowing
    /// its size, and `poll_ready` doesn't tell that.
    next_request: Option<T>,

    /// A batch that is currently being constructed or has already been
    /// constructed (in which case it's said to be *sealed*). It may be left in
    /// the sealed state (`next_batch_bind` is `Some(_)`) if we can't bind
    /// the staging buffer to `heap` because `heap` is full.
    next_batch: Vec<(T, Range<DeviceSize>)>,
    next_batch_size: DeviceSize,
    next_batch_bind: Option<(Bind, base::BufferRef)>,

    /// A queue of batches that have already been submitted to the device.
    batch_ring: BatchRing<T>,

    heap: H,
    cmd_generator: G,
}

impl<T, H, G> Streamer<T, H, G>
where
    T: Unpin + Request,
    H: Unpin + Borrow<AsyncHeap>,
    G: Unpin + CmdGenerator<T>,
{
    pub fn new(params: Builder<G>, heap: H) -> Self {
        Self {
            device: params.device,
            queue: params.queue,
            max_batch_size: params.batch_size as DeviceSize,
            should_wait_completion: params.should_wait_completion,
            next_request: None,
            next_batch: Vec::new(),
            next_batch_size: 0,
            next_batch_bind: None,
            batch_ring: BatchRing::new(),
            heap,
            cmd_generator: params.cmd_generator,
        }
    }

    /// Make `next_request` vacant.
    fn try_dispatch_request(&mut self) -> bool {
        if self.next_request.is_some() {
            // Can't modify a sealed batch
            if self.next_batch_bind.is_some() {
                return false;
            }

            let remaining = self.max_batch_size - self.next_batch_size;
            let next_request_size = self.next_request.as_ref().unwrap().size() as DeviceSize;

            if next_request_size > remaining {
                return false;
            }

            let buffer_range = self.next_batch_size..self.next_batch_size + next_request_size;

            self.next_batch
                .push((self.next_request.take().unwrap(), buffer_range));
            self.next_batch_size += next_request_size;
        }

        true
    }

    /// Submit `next_batch.*` and make it empty.
    fn poll_dispatch_batch(&mut self, cx: &task::LocalWaker) -> Poll<Result<()>> {
        if self.next_batch.is_empty() {
            return Ok(()).into();
        }

        // Seal the batch
        if self.next_batch_bind.is_none() {
            let usage: base::BufferUsageFlags = (self.next_batch.iter())
                .map(|req| req.0.staging_buffer_usage())
                .collect();
            let buffer = (self.device.build_buffer())
                .size(self.next_batch_size)
                .usage(usage)
                .queue(&self.queue)
                .build()?;
            let bind = self.heap.borrow().bind((&buffer).into());
            self.next_batch_bind = Some((bind, buffer));
        }

        // Wait until the staging buffer is ready
        {
            let ref mut bind = self.next_batch_bind.as_mut().unwrap().0;
            try_ready!(bind.poll_unpin(cx));
        }

        // Now we know that `next_batch` is ready to submit, so...
        use std::mem::replace;

        let buffer = self.next_batch_bind.take().unwrap().1;
        let buffer_size = self.next_batch_size;
        let mut requests = replace(&mut self.next_batch, Vec::new());
        self.next_batch_size = 0;

        // Fill the staging buffer
        {
            use std::slice::from_raw_parts_mut;
            let ptr = buffer.as_ptr();
            let slice = unsafe { from_raw_parts_mut(ptr, buffer_size as _) };
            for (request, range) in &mut requests {
                let range = range.start as _..range.end as _;
                request.populate(&mut slice[range]);
            }
        }

        // Consturct a command buffer and then submit it away
        let mut cmd_buffer = self.queue.new_cmd_buffer()?;

        self.cmd_generator
            .encode(&mut cmd_buffer, &buffer, &mut requests)?;

        // Submit the command buffer
        let cb_result = cmd_buffer.result();
        cmd_buffer.commit()?;

        self.queue.flush();

        self.batch_ring.queue.push_back(Batch {
            cb_result,
            requests,
            buffer,
        });

        Ok(()).into()
    }

    fn poll_flush_inner(
        mut self: Pin<&mut Self>,
        cx: &task::LocalWaker,
        should_wait_completion: bool,
    ) -> Poll<Result<()>> {
        let this = &mut *self;
        assert_eq!(
            this.batch_ring.poll_flush(this.heap.borrow(), false, cx)?,
            Poll::Ready(())
        );

        if !this.try_dispatch_request() {
            try_ready!(this.poll_dispatch_batch(cx));
        }
        assert!(this.try_dispatch_request());
        try_ready!(this.poll_dispatch_batch(cx));

        this.batch_ring
            .poll_flush(this.heap.borrow(), should_wait_completion, cx)
    }
}

impl<T, H, G> Sink for Streamer<T, H, G>
where
    T: Unpin + Request,
    H: Unpin + Borrow<AsyncHeap>,
    G: Unpin + CmdGenerator<T>,
{
    type SinkItem = T;
    type SinkError = base::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &task::LocalWaker) -> Poll<Result<()>> {
        let this = &mut *self;
        assert_eq!(
            this.batch_ring.poll_flush(this.heap.borrow(), false, cx)?,
            Poll::Ready(())
        );

        if this.try_dispatch_request() {
            return Ok(()).into();
        }

        // When `poll_dispatch_batch` returns `Async::Pending`, it indicates
        // that there is no room in the staging buffer heap to start a new
        // batch.
        //
        // In this case, `poll_dispatch_batch` is responsible for maintaining a
        // `Waker` object so the current task can be waken up later when a room
        // becomes available. However, it won't happen if it's *us* which are
        // also making the heap full.
        //
        // This is why we call `self.batch_ring.poll_flush` first. It internally
        // calls `poll` on `CmdBufferResult`s, ensuring the current task is
        // woken up upon command buffer completion to release the associated
        // staging buffer.
        try_ready!(this.poll_dispatch_batch(cx));
        assert!(this.try_dispatch_request());
        Ok(()).into()
    }

    fn start_send(mut self: Pin<&mut Self>, item: Self::SinkItem) -> Result<()> {
        assert!(self.next_request.is_none());
        self.next_request = Some(item);
        Ok(())
    }

    /// Flush any remaining requests. The behavior of flushing is dependent on
    /// the value of [`Builder::should_wait_completion`].
    fn poll_flush(self: Pin<&mut Self>, cx: &task::LocalWaker) -> Poll<Result<()>> {
        let should_wait_completion = self.should_wait_completion;
        self.poll_flush_inner(cx, should_wait_completion)
    }

    /// Flush any remaining requests and wait for the completion of all
    /// associated command buffers (no matter what value
    /// `should_wait_completion` is set to).
    fn poll_close(self: Pin<&mut Self>, cx: &task::LocalWaker) -> Poll<Result<()>> {
        self.poll_flush_inner(cx, true)
    }
}

#[derive(Debug)]
struct BatchRing<T> {
    /// A FIFO queue of unfinished batches.
    queue: VecDeque<Batch<T>>,
}

#[derive(Debug)]
struct Batch<T> {
    cb_result: CmdBufferResult,

    requests: Vec<(T, Range<DeviceSize>)>,

    /// The staging buffer for this batch. Should be returned to `Streamer::heap`
    /// after the completion.
    buffer: base::BufferRef,
}

impl<T: Request> BatchRing<T> {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    /// Check command buffer completion. Finalize completed batches and
    /// deallocate their staging buffers.
    ///
    /// Returns `Ready` if the queue is empty or `should_wait_completion` is
    /// `false`.
    fn poll_flush(
        &mut self,
        heap: &AsyncHeap,
        should_wait_completion: bool,
        cx: &task::LocalWaker,
    ) -> Poll<Result<()>> {
        while self.queue.len() > 0 {
            {
                let front: &mut Batch<_> = self.queue.front_mut().unwrap();

                match Pin::new(&mut front.cb_result).poll(cx) {
                    Poll::Ready(Err(_)) => panic!("CB cancelled unexpectedly"),
                    // CB submission failure is fatal
                    Poll::Ready(Ok(result)) => result?,
                    // This CB being in progress means all of the rest of CBs
                    // aren't completed yet
                    Poll::Pending => {
                        if should_wait_completion {
                            return Poll::Pending;
                        } else {
                            return Ok(()).into();
                        }
                    }
                }

                let data = front.buffer.as_bytes_volatile();
                for (req, range) in front.requests.iter_mut() {
                    let range = range.start as _..range.end as _;
                    req.exfiltrate(&data[range]);
                }

                heap.make_aliasable((&front.buffer).into())?;
            }
            self.queue.pop_front().unwrap();
        }
        Ok(()).into()
    }
}
