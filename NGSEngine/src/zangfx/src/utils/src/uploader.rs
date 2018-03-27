//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Uploads immutable data to the device memory.
//!
//! Uploading is done in a unit named *session*, which consists of one or
//! more resources. Each session is associated with a staging buffer in a
//! host-visible heap.
//!
//! Uploading is done in the following steps:
//!
//! 1. `Uploader` collects upload requests. An upload request consists of the
//!    number of bytes allocated from the staging buffer, a function
//!    (*populate function*) used to populate the staging buffer with the
//!    uploaded data, and another one (*copy function*) for encoding copy
//!    commands.
//!
//! 2. `Uploader` sorts the received upload requests into groups called
//!    *sessions*. For each session, a staging buffer is allocated and filled
//!    with contents using the *populate functions* associated with the upload
//!    requests in the session.
//!
//! 3. For each session, a command buffer is created with copy commands encoded
//!    by the *copy functions* associated with the upload requests in the
//!    session.
//!
//!    Each command buffer also contains a fence signal operation. Furthermore,
//!    sessions are given a sequencially ordering defined by fences between
//!    adjacent sessions, so the application would only have to wait on the
//!    session with the largest sequencial number.
//!
//! 4. The command buffers are submitted to the queue.
//!
//! 5. Whenever `recycle` is called, `Uploader` checks the state of the
//!    submitted command buffers. Staging buffers are returned to the heap upon
//!    the retirement of their associated upload sessions.
//!
use std::sync::Arc;
use std::collections::VecDeque;
use std::ops::Range;
use itertools::unfold;

use base::{self, Result};
use base::prelude::*;
use cbstatetracker::CbStateTracker;
use smartref::UniqueBuffer;
use DeviceUtils;

pub use uploaderutils::*;

/// Represents a session ID of `Uploader`.
///
/// Session IDs are chronologically ordered and start with `1`, and increase by
/// `1` whenever a new session is created.
pub type SessionId = u64;

/// Parameters for `Uploader`.
#[derive(Debug, Clone)]
pub struct UploaderParams {
    pub device: Arc<base::Device>,

    pub queue: Arc<base::CmdQueue>,

    /// The maximum number of bytes transferred per session.
    pub max_bytes_per_session: usize,

    /// The maximum number of total bytes of ongoing upload sessions.
    pub max_bytes_ongoing: usize,
}

/// An upload request consumed by `Uploader`.
pub trait UploadRequest {
    /// The number of bytes required in the staging buffer.
    fn size(&self) -> usize;

    /// Fill the staging buffer with the contents.
    fn populate(&self, staging_buffer: &mut [u8]);

    /// Encode copy commands.
    fn copy(
        &self,
        _encoder: &mut base::CopyCmdEncoder,
        _staging_buffer: &base::Buffer,
        _staging_buffer_range: Range<base::DeviceSize>,
    ) -> Result<()> {
        Ok(())
    }

    /// Encode copy commands. Called after `copy` is called for all requests in
    /// the same session.
    fn post_copy(
        &self,
        _encoder: &mut base::CopyCmdEncoder,
        _staging_buffer: &base::Buffer,
        _staging_buffer_range: Range<base::DeviceSize>,
    ) -> Result<()> {
        Ok(())
    }

    /// Encode commands outside a command encoder.
    fn post_encoder(&self, _cmd_buffer: &mut base::CmdBuffer) -> Result<()> {
        Ok(())
    }

    /// Retrieves the precise session ID.
    fn hook_session(&self, _session_id: SessionId) {}
}

/// Uploads immutable data to the device memory.
///
/// See the module-level documentation for details.
#[derive(Debug)]
pub struct Uploader {
    device: Arc<base::Device>,
    queue: Arc<base::CmdQueue>,
    cmd_pool: Box<base::CmdPool>,
    /// The dynamic heap from which staging buffers are allocated from.
    heap: Box<base::Heap>,
    empty_barrier: base::Barrier,

    max_bytes_per_session: usize,
    max_bytes_ongoing: usize,

    sessions: SessionRing,
}

impl Drop for Uploader {
    fn drop(&mut self) {
        self.queue.flush();
        let _ = self.wait();
    }
}

impl Uploader {
    pub fn new(params: UploaderParams) -> Result<Self> {
        let device = params.device;
        let queue = params.queue;

        let cmd_pool = queue.new_cmd_pool()?;

        let heap = device
            .build_dynamic_heap()
            .memory_type(
                device
                    .memory_type_for_buffer(
                        flags![base::BufferUsage::{CopyRead}],
                        flags![base::MemoryTypeCaps::{HostVisible | HostCoherent}],
                        flags![base::MemoryTypeCaps::{HostVisible | HostCoherent}],
                    )?
                    .unwrap(),
            )
            .size(params.max_bytes_ongoing as u64)
            .build()?;

        let empty_barrier = device.build_barrier().build()?;

        Ok(Self {
            device,
            queue,
            cmd_pool,
            heap,
            empty_barrier,
            max_bytes_per_session: params.max_bytes_per_session,
            max_bytes_ongoing: params.max_bytes_ongoing,
            sessions: SessionRing::new(),
        })
    }

    pub fn device(&self) -> &Arc<base::Device> {
        &self.device
    }

    pub fn queue(&self) -> &Arc<base::CmdQueue> {
        &self.queue
    }

    /// Check the completion of sessions.
    pub fn recycle(&mut self) -> Result<()> {
        self.sessions.recycle(&*self.device, &*self.heap)?;
        Ok(())
    }

    /// Wait until all ongoing sessions are retired.
    ///
    /// Warning: This method does not call `CmdQueue::flush` by itself. It will
    /// dead-lock if there are any session that is not submitted to the device
    /// yet.
    ///
    /// A following call to `num_ongoing_sessions` will return `0`.
    pub fn wait(&mut self) -> Result<()> {
        self.sessions.wait();
        self.recycle()
    }

    /// Wait until the specified and all preceding sessions are retired.
    ///
    /// Warning: This method does not call `CmdQueue::flush` by itself. It will
    /// dead-lock if there are any session that is not submitted to the device
    /// yet.
    pub fn wait_until_session(&mut self, session: SessionId) -> Result<()> {
        self.sessions.wait_until_session(session);
        self.recycle()
    }

    /// Get the number of ongoing sessions. Call `recylce` first to get the
    /// up-to-date information.
    pub fn num_ongoing_sessions(&self) -> usize {
        self.sessions.sessions.len()
    }

    /// Get the fence to be updated by the specified session.
    ///
    /// # Panics
    ///
    ///  - The session with the specified ID has not been created yet.
    pub fn get_fence(&self, session: SessionId) -> Option<&base::Fence> {
        if session < self.sessions.session_start_id {
            self.sessions.last_fence.as_ref()
        } else {
            self.sessions
                .sessions
                .get((session - self.sessions.session_start_id) as usize)
                .map(|session| Some(&session.fence))
                .expect("invalid session ID")
        }
    }

    /// Start zero or more upload sessions.
    ///
    /// Returns the last session ID used to fulfill the requests. Alternatively,
    /// you can implement `UploadRequest::hook_session` to know the precise
    /// session ID for each request.
    ///
    /// This method might introduce a CPU stall if there are too many ongoing
    /// sessions and there is no enough room in the staging buffer heap. Use
    /// `num_ongoing_sessions` to meter the uploading speed.
    ///
    /// This method does not flush the queue after commiting a command buffer.
    ///
    /// # Panics
    ///
    ///  - At least one of the requests requires a staging buffer larger than
    ///    `UploaderParams::max_bytes_ongoing`.
    pub fn upload<I, T>(&mut self, mut requests: I) -> Result<SessionId>
    where
        I: Iterator<Item = T> + Clone,
        T: UploadRequest,
    {
        let mut last_session_id = self.sessions.last_session_id();

        loop {
            let start = requests.clone();
            let this_session_id = last_session_id + 1;

            // Split the requests into sessions using the maximum bytes.
            //
            // A session is allowed to be larger than `max_bytes_per_session`
            // if some requests are very large. However, no request can be
            // larger than `max_bytes_ongoing`.
            let mut session_size: usize = 0;
            let mut count: usize = 0;
            let mut cur = requests.clone();
            while let Some(request) = cur.next() {
                let size = request.size();
                if session_size + size > self.max_bytes_per_session && count > 0 {
                    break;
                }
                assert!(size <= self.max_bytes_ongoing);
                session_size += size;
                count += 1;
                requests = cur.clone();
            }

            if count == 0 {
                // That's all!
                break;
            }

            let sub_requests = start.take(count);

            // The layout of the staging buffer
            // `() -> impl Iterator<Item = (T, Range<usize>)>`
            let sub_requests_with_range = || {
                unfold(
                    (0usize, sub_requests.clone()),
                    |&mut (ref mut next_offset, ref mut it)| {
                        it.next().map(|request| {
                            let offset = *next_offset;
                            let size = request.size();
                            *next_offset += size;
                            (request, offset..offset + size)
                        })
                    },
                )
            };

            // Suballocate the staging buffer
            let buffer = self.device
                .build_buffer()
                .size(session_size as _)
                .usage(flags![base::BufferUsage::{CopyRead}])
                .build()?;
            let buffer = UniqueBuffer::new(&*self.device, buffer);

            let alloc = loop {
                macro_rules! try_bind {
                    () => {
                        if let Some(alloc) = self.heap.bind((&*buffer).into())? {
                            break alloc;
                        }
                    }
                }
                macro_rules! try_recycle {
                    () => {
                        if self.sessions.recycle(&*self.device, &*self.heap)? {
                            // Now that some sessions are recycled, there may
                            // be a room in the staging bufefr heap.
                            try_bind!();
                        }
                    }
                }

                try_bind!();

                try_recycle!();

                self.queue.flush();
                try_recycle!();

                self.sessions.wait();
                try_recycle!();

                unreachable!();
            };

            // Populate the staging buffer
            {
                use std::slice::from_raw_parts_mut;
                let ptr = self.heap.as_ptr(&alloc)?;
                let slice = unsafe { from_raw_parts_mut(ptr, session_size) };
                for (request, range) in sub_requests_with_range() {
                    request.populate(&mut slice[range]);
                }
            }

            // Construct the command buffer
            let mut cmd_buffer: base::SafeCmdBuffer = self.cmd_pool.begin_cmd_buffer()?;
            let fence = self.queue.new_fence()?;
            {
                let encoder = cmd_buffer.encode_copy();
                if let Some(ref fence) = self.sessions.last_fence {
                    // Enforce ordering
                    encoder.wait_fence(fence, flags![base::Stage::{Copy}], &self.empty_barrier);
                }

                // Encode copy commands
                for (request, range) in sub_requests_with_range() {
                    let range = range.start as u64..range.end as u64;
                    request.copy(encoder, &*buffer, range)?;
                }
                for (request, range) in sub_requests_with_range() {
                    let range = range.start as u64..range.end as u64;
                    request.post_copy(encoder, &*buffer, range)?;
                }

                encoder.update_fence(&fence, flags![base::Stage::{Copy}]);
            }
            for request in sub_requests.clone() {
                request.post_encoder(&mut *cmd_buffer)?;
            }

            // Install the command buffer state tracker on it
            let cb_state_tracker = CbStateTracker::new(&mut *cmd_buffer);

            // Submit away
            cmd_buffer.commit()?;

            self.sessions.sessions.reserve(1);
            self.sessions.sessions.push_back(Session {
                cb_state_tracker,
                fence,
                alloc,
                buffer: buffer.into_inner().1,
            });

            for request in sub_requests.clone() {
                request.hook_session(this_session_id);
            }

            last_session_id = this_session_id;
        }

        Ok(last_session_id)
    }
}

#[derive(Debug)]
struct SessionRing {
    /// The session ID offset of `sessions`.
    session_start_id: SessionId,
    /// Ongoing session FIFO.
    sessions: VecDeque<Session>,
    /// The fence signaled by the last retired session.
    last_fence: Option<base::Fence>,
}

#[derive(Debug)]
struct Session {
    /// Tracks the state of the command buffer of the session.
    cb_state_tracker: CbStateTracker,
    /// The fence signaled on the completion of the session.
    fence: base::Fence,
    /// An allocation from `Uploader::heap`.
    alloc: base::HeapAlloc,
    /// The staging buffer.
    buffer: base::Buffer,
}

impl SessionRing {
    fn new() -> Self {
        Self {
            session_start_id: 1,
            sessions: VecDeque::new(),
            last_fence: None,
        }
    }

    /// The session ID of the last session (retired or not).
    fn last_session_id(&self) -> SessionId {
        self.session_start_id + (self.sessions.len() as u64) - 1
    }

    /// Check the completion of sessions. Returns a flag indicating whether at
    /// least one session has retired or not.
    fn recycle(&mut self, device: &base::Device, heap: &base::Heap) -> Result<bool> {
        let mut some_retired = false;

        while self.sessions.len() > 0 && self.sessions[0].cb_state_tracker.is_completed() {
            some_retired = true;

            // Retire the session
            let session = self.sessions.pop_front().unwrap();
            self.session_start_id += 1;

            device.destroy_buffer(&session.buffer)?;
            heap.unbind(&session.alloc)?;
            self.last_fence = Some(session.fence);
        }

        Ok(some_retired)
    }

    fn wait(&mut self) {
        for session in self.sessions.iter() {
            session.cb_state_tracker.wait();
        }
    }

    fn wait_until_session(&mut self, session_id: SessionId) {
        let mut cur_id = self.session_start_id;
        for session in self.sessions.iter() {
            if cur_id > session_id {
                break;
            }
            session.cb_state_tracker.wait();
            cur_id += 1;
        }
    }
}
