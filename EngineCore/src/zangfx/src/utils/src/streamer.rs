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
//! - *Phases* are used to define explicit ordering among device commands from
//!   multiple requests inside a single batch.
//!
//! # Basic operations
//!
//! [`Streamer`] is a `Sink` accepting a stream of user-supplied
//! `impl `[`StreamerRequest`]s each representing a *request*.
//!
//! The streaming operation is performed in a unit called a *batch* each
//! composed of one or more requests.
//! For each request, a portion of the staging buffer whose size is specified
//! via [`size`] is allocated, [`populate`] is called to initialize the
//! contents of the allocated portion, and the request is added to the current
//! batch.
//! This step is repeated until (1) the total amount of portions allocated for
//! the current batch reaches the maximum batch size specified via
//! [`StreamerParams::batch_size`] or (2) there are no more requests to process.
//! At this point, the batch is said to be *sealed*.
//!
//! [`Streamer`]: crate::streamer::Streamer
//! [`StreamerRequest`]: crate::streamer::StreamerRequest
//! [`size`]: crate::streamer::StreamerRequest::size
//! [`populate`]: crate::streamer::StreamerRequest::populate
//! [`StreamerParams::batch_size`]: crate::streamer::StreamerParams::batch_size
//!
//! After a batch is sealed, a command buffer is constructed for that batch.
//! Device commands are encoded using [`copy`] and [`outside_encoder`] for every
//! request in the batch in question. After that, the command buffer is queued
//! for execution. `CmdQueue` is automatically flushed.
//!
//! [`copy`]: crate::streamer::StreamerRequest::copy
//! [`outside_encoder`]: crate::streamer::StreamerRequest::outside_encoder
//!
//! Upon command buffer completion, [`exfiltrate`] is called to give requests a
//! chance to extract the data stored in the staging buffer (for device-to-host
//! transfer). Finally, the allocated portions of the staging buffer are
//! released.
//!
//! [`exfiltrate`]: crate::streamer::StreamerRequest::exfiltrate
//!
//! # Phases
//!
//! *Phases* can be used to define explicit ordering among device commands
//! generated from different requests in a single batch. Each phase is
//! identified by a `u32` in range `0..32`.
//!
//! Each phase can execute one of the following encoder types: `copy` and
//! `outside_encoder`. It's an error for more than one encoder type to occupy
//! a single phase. For example, since `copy` occupies the 16th phase by
//! default, you might want to avoid choosing the same phase for
//! `outside_encoder` (unless you are sure that `copy` doesn't run during the
//! 16th phase).
//!
//! Consecutive phases with an identical encoder type are *not* merged into
//! a single command encoder.
//!
//! Here's an example of a phase arrangement:
//!
//!  - Encode copy commands in the `16`th phase (the default for `copy`).
//!  - Encode *release queue family ownership operations* (which only can be
//!    encoded outside an encoder) in the `24`th phase.
//!    To do this, write `fn outside_encoder_phase_set() -> u32 { 1 << 24 }`
//!    and implement `outside_encoder` on your request type.
//!
//! # Error handling
//!
//! `Streamer` only handles fatal error conditions such as device loss.
//! Implementors of `StreamerRequest` can only return the errors relevant to the
//! operation of `Streamer`, and should handle other kinds of errors through
//! other means.
use futures::{task, try_ready, Async, Future, Sink};
use ngsenumflags::flags;
use std::{borrow::Borrow, collections::VecDeque, ops::Range};
use volatile_view::Volatile;

use zangfx_base::{self as base, DeviceSize, Result};
use zangfx_common::BinaryInteger;

use crate::{
    asyncheap::{AsyncHeap, Bind},
    buffer::BufferUtils,
    futuresapi::{CmdBufferFutureExt, CmdBufferResult},
};

/// Parameters for `Streamer`.
#[derive(Debug, Clone)]
pub struct StreamerParams {
    pub device: base::DeviceRef,

    pub queue: base::CmdQueueRef,

    /// The maximum number of bytes transferred per batch.
    pub batch_size: usize,

    /// Specifies whether flushing requires that all command buffers are
    /// completed.
    ///
    /// When set to `false`, flushing only ensures that the command buffer
    /// has been submitted. `Streamer` might not check the command buffer
    /// completion at all, which have some ramifications:
    ///
    /// - You won't be able to use `StreamerRequest::exfiltrate`.
    /// - Necessitates uses of graphics API-level synchronization primitives
    ///   such as *semaphores* (for inter-queue synchronization) and *fences*
    ///   (for intra-queue synchronization).
    /// - `Streamer` won't release staging buffers after flushing, which
    ///   impades using multiple `Streamer`s sharing a single `AsyncHeap`.
    ///
    pub should_wait_completion: bool,
}

/// A request to be processed by [`Streamer`].
pub trait StreamerRequest {
    /// The number of bytes required in the staging buffer.
    fn size(&self) -> usize;

    /// Fill the staging buffer with the contents.
    fn populate(&mut self, _staging_buffer: &mut [u8]) {}

    /// Get the usage flags required for the staging buffer.
    fn staging_buffer_usage(&self) -> base::BufferUsageFlags {
        flags![base::BufferUsage::{CopyRead}]
    }

    /// Return a bit array where each bit represents whether `copy` should be
    /// called during the phase corresponding to the bit position.
    ///
    /// The default implementation returns `1 << 16`, which indicates that
    /// `copy` has to be called during the 16th phase.
    ///
    /// # Examples
    ///
    ///  - The default value `1 << 16` causes `copy` to called during the 16th
    ///    phase.
    ///  - `(1 << 16) | (1 << 4)` causes `copy` to be called twice with `phase`
    ///    set to `4` and `16` respectively.
    ///
    fn copy_phase_set(&self) -> u32 {
        1 << 16
    }

    /// Encode copy commands.
    fn copy(
        &mut self,
        _encoder: &mut dyn base::CopyCmdEncoder,
        _staging_buffer: &base::BufferRef,
        _staging_buffer_range: Range<DeviceSize>,
        _phase: u32,
    ) -> Result<()> {
        Ok(())
    }

    /// Return a bit array where each bit represents whether `outside_encoder`
    /// should be called during the phase corresponding to the bit position.
    ///
    /// The default implementation returns `0`, which indicates that
    /// `outside_encoder` should not be called.
    fn outside_encoder_phase_set(&self) -> u32 {
        0
    }

    /// Encode commands outside a command encoder.
    fn outside_encoder(
        &mut self,
        _cmd_buffer: &mut base::CmdBufferRef,
        _staging_buffer: &base::BufferRef,
        _staging_buffer_range: Range<DeviceSize>,
        _phase: u32,
    ) -> Result<()> {
        unreachable!()
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
///  - `T: `[`StreamerRequest`] - A type representing requests.
///  - `H: Borrow<`[`AsyncHeap`]`>` - `AsyncHeap` or something that can be used
///    to borrow a reference to `AsyncHeap`. A value of this type is supplied at
///    construction time. Staging buffers are allocated from that.
///
/// [`AsyncHeap`]: crate::asyncheap::AsyncHeap
///
#[derive(Debug)]
pub struct Streamer<T, H> {
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
    /// the sealed state (`next_batch_bind` is `Some(_)`) if we can't move it
    /// to the next stage for `heap` being full.
    next_batch: Vec<(T, Range<DeviceSize>)>,
    next_batch_size: DeviceSize,
    next_batch_bind: Option<(Bind, base::BufferRef)>,

    /// A queue of batches that have already been submitted to the device.
    batch_ring: BatchRing<T>,

    heap: H,
}

impl<T: StreamerRequest, H: Borrow<AsyncHeap>> Streamer<T, H> {
    pub fn new(params: StreamerParams, heap: H) -> Self {
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
    fn poll_dispatch_batch(&mut self, cx: &mut task::Context<'_>) -> Result<Async<()>> {
        if self.next_batch.is_empty() {
            return Ok(Async::Ready(()));
        }

        // Seal the batch
        if self.next_batch_bind.is_none() {
            let usage: base::BufferUsageFlags = (self.next_batch.iter())
                .map(|req| req.0.staging_buffer_usage())
                .collect();
            let buffer = (self.device.build_buffer())
                .size(self.next_batch_size)
                .usage(usage)
                .build()?;
            let bind = self.heap.borrow().bind((&buffer).into());
            self.next_batch_bind = Some((bind, buffer));
        }

        // Wait until the staging buffer is ready
        {
            let ref mut bind = self.next_batch_bind.as_mut().unwrap().0;
            try_ready!(bind.poll(cx));
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

        let copy_phase_set = requests
            .iter()
            .map(|req| req.0.copy_phase_set())
            .fold(0, |x, y| x | y);
        let outside_encoder_phase_set = requests
            .iter()
            .map(|req| req.0.outside_encoder_phase_set())
            .fold(0, |x, y| x | y);

        assert!(
            copy_phase_set & outside_encoder_phase_set == 0,
            "No phase slot can have more than one encoder type"
        );

        let phase_set = copy_phase_set | outside_encoder_phase_set;

        for phase in phase_set.one_digits() {
            if copy_phase_set.get_bit(phase) {
                let encoder = cmd_buffer.encode_copy();
                for (request, range) in &mut requests {
                    if request.copy_phase_set().get_bit(phase) {
                        request.copy(encoder, &buffer, range.clone(), phase)?;
                    }
                }
            }
            if outside_encoder_phase_set.get_bit(phase) {
                for (request, range) in &mut requests {
                    if request.outside_encoder_phase_set().get_bit(phase) {
                        request.outside_encoder(&mut cmd_buffer, &buffer, range.clone(), phase)?;
                    }
                }
            }
        }

        // Submit the command buffer
        let cb_result = cmd_buffer.result();
        cmd_buffer.commit()?;

        self.queue.flush();

        self.batch_ring.queue.push_back(Batch {
            cb_result,
            requests,
            buffer,
        });

        Ok(Async::Ready(()))
    }
}

impl<T: StreamerRequest, H: Borrow<AsyncHeap>> Sink for Streamer<T, H> {
    type SinkItem = T;
    type SinkError = base::Error;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Result<Async<()>> {
        try_ready!(self.batch_ring.poll_flush(self.heap.borrow(), false, cx));

        if self.try_dispatch_request() {
            return Ok(Async::Ready(()));
        }
        try_ready!(self.poll_dispatch_batch(cx));
        assert!(self.try_dispatch_request());
        Ok(Async::Ready(()))
    }

    fn start_send(&mut self, item: Self::SinkItem) -> Result<()> {
        assert!(self.next_request.is_none());
        self.next_request = Some(item);
        Ok(())
    }

    fn poll_flush(&mut self, cx: &mut task::Context<'_>) -> Result<Async<()>> {
        try_ready!(self.batch_ring.poll_flush(self.heap.borrow(), false, cx));

        if !self.try_dispatch_request() {
            try_ready!(self.poll_dispatch_batch(cx));
        }
        assert!(self.try_dispatch_request());
        try_ready!(self.poll_dispatch_batch(cx));

        self.batch_ring
            .poll_flush(self.heap.borrow(), self.should_wait_completion, cx)
    }

    fn poll_close(&mut self, cx: &mut task::Context<'_>) -> Result<Async<()>> {
        self.poll_flush(cx)
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

impl<T: StreamerRequest> BatchRing<T> {
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
        cx: &mut task::Context<'_>,
    ) -> Result<Async<()>> {
        while self.queue.len() > 0 {
            {
                let front: &mut Batch<_> = self.queue.front_mut().unwrap();

                match front.cb_result.poll(cx).expect("CB cancelled unexpectedly") {
                    // CB submission failure is fatal
                    Async::Ready(result) => result?,
                    // This CB being in progress means all of the rest of CBs
                    // aren't completed yet
                    Async::Pending => {
                        if should_wait_completion {
                            return Ok(Async::Pending);
                        } else {
                            return Ok(Async::Ready(()));
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
        Ok(Async::Ready(()))
    }
}
