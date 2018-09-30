//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides [`AsyncHeap`], a `Future`-compatible wrapper of `Heap` that blocks
//! if there isn't a sufficient space.
//!
//! [`AsyncHeap`]: asyncheap::AsyncHeap
use futures::{task, Async, Future};
use parking_lot::Mutex;
use std::{collections::VecDeque, sync::Arc};

use zangfx_base::{self as base, Error, HeapRef, ResourceRef, Result};

/// A `Future`-compatible wrapper of `Heap` that blocks if there isn't a
/// sufficient space.
#[derive(Debug)]
pub struct AsyncHeap {
    heap: HeapRef,
    inner: Arc<Mutex<Inner>>,
}

/// An `Future` representing a `bind` operation on a [`AsyncHeap`].
///
/// The result of dropping this too early isn't specified.
#[derive(Debug)]
pub struct Bind(BindState);

#[derive(Debug)]
enum BindState {
    /// The operation haven't started yet.
    Initial {
        resource: Resource,
        inner: Arc<Mutex<Inner>>,
    },
    /// The operation is under way.
    Pending(Arc<Item>),
    /// We already returned the result.
    Done,
}

#[derive(Debug)]
struct Item {
    waker: Mutex<task::Waker>,
    resource: Resource,
    /// Written once when the operation is done (successfully or not) and
    /// read once when `Bind::poll` is called, or never if `Bind` was dropped
    /// too early. Note that `zangfx::base::Result<_>` is not `Clone`.
    result: Mutex<Option<Result<()>>>,
}

#[derive(Debug)]
struct Inner {
    heap: HeapRef,
    queue: VecDeque<Arc<Item>>,
}

impl AsyncHeap {
    /// Construct an `AsyncHeap` by wrapping a supplied `HeapRef`.
    pub fn new(heap: HeapRef) -> Self {
        let inner = Inner {
            heap: heap.clone(),
            queue: VecDeque::new(),
        };

        Self {
            heap,
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// Get a reference to the inner `Heap` object.
    ///
    /// Note that the clients of `AsyncHeap` won't get woken up if you call
    /// the contained `HeapRef`'s `Heap::make_aliasable` directly.
    pub fn heap(&self) -> &HeapRef {
        &self.heap
    }

    /// Allocate a memory region for a given resource. Wait if there isn't a
    /// sufficient space yet.
    ///
    /// This method returns a `Future<Item = (), Error = zangfx::base::Error>`,
    /// reflecting the return type of `Heap::bind` except that there isn't a
    /// `bool` value indicating whether the allocation was successful, since
    /// this `bind` completes only when the allocation was successful.
    ///
    /// Allocations are always processed in a FIFO (first-in-first-out) fashion.
    ///
    /// **Warning**: Make sure the resource's memory requirement is
    /// smaller than the heap. Otherwise, the allocation queue would get
    /// stuck forever because the allocation would never succeed.
    ///
    /// # Valid Usage
    ///
    /// See `Heap`'s documentation.
    pub fn bind(&self, obj: ResourceRef<'_>) -> Bind {
        Bind(BindState::Initial {
            resource: Resource::clone_from(obj),
            inner: Arc::clone(&self.inner),
        })
    }

    /// Mark the allocated region available for future allocations.
    ///
    /// # Valid Usage
    ///
    /// See `Heap`'s documentation.
    pub fn make_aliasable(&self, obj: ResourceRef<'_>) -> Result<()> {
        self.heap.make_aliasable(obj)?;

        // Process pending bind requests
        let mut inner = self.inner.lock();
        let ref mut inner = *inner; // enable split borrow

        while inner.queue.len() > 0 {
            {
                let item = inner.queue.front().unwrap();

                let result = match inner.heap.bind(item.resource.as_ref()) {
                    Ok(true) => Ok(()),
                    Ok(false) => break,
                    Err(x) => Err(x),
                };

                *item.result.lock() = Some(result);
                item.waker.lock().wake();
            }

            inner.queue.pop_front();
        }

        Ok(())
    }
}

impl Future for Bind {
    type Item = ();
    type Error = Error;

    fn poll(
        &mut self,
        cx: &mut task::Context<'_>,
    ) -> std::result::Result<Async<Self::Item>, Self::Error> {
        use std::mem::replace;

        match replace(&mut self.0, BindState::Done) {
            BindState::Done => unreachable!(),
            BindState::Initial { resource, inner } => {
                let mut inner = inner.lock();

                // If the queue is empty, we can try the inner `Heap` right now
                // and maybe we can return without constructing an `Item`.
                // Otherwise, doing so would break our FIFO scheme.
                if inner.queue.len() == 0 {
                    // Errors returned by a `Heap` is usually fatal, so return
                    // `Err(_)` immediately if we get that from the inner `bind`
                    if inner.heap.bind(resource.as_ref())? {
                        return Ok(Async::Ready(()));
                    }
                }

                let item = Arc::new(Item {
                    waker: Mutex::new(cx.waker().clone()),
                    resource,
                    result: Mutex::new(None),
                });

                inner.queue.push_back(Arc::clone(&item));
                drop(inner);

                self.0 = BindState::Pending(item);
                Ok(Async::Pending)
            }
            BindState::Pending(item) => {
                if let Some(result) = item.result.lock().take() {
                    Ok(Async::Ready(result?))
                } else {
                    let mut waker = item.waker.lock();
                    if !waker.will_wake(cx.waker()) {
                        *waker = cx.waker().clone();
                    }
                    Ok(Async::Pending)
                }
            }
        }
    }
}

// FIXME: There are too many duplicates of this
#[derive(Debug, Clone)]
enum Resource {
    Image(base::ImageRef),
    Buffer(base::BufferRef),
}

impl Resource {
    fn clone_from(x: base::ResourceRef<'_>) -> Self {
        match x {
            base::ResourceRef::Image(x) => Resource::Image(x.clone()),
            base::ResourceRef::Buffer(x) => Resource::Buffer(x.clone()),
        }
    }

    fn as_ref(&self) -> base::ResourceRef<'_> {
        match self {
            Resource::Image(ref x) => base::ResourceRef::Image(x),
            Resource::Buffer(ref x) => base::ResourceRef::Buffer(x),
        }
    }
}
