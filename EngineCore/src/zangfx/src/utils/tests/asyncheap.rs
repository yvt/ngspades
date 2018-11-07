//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use futures::{executor::block_on, future};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use zangfx_base::{self as base, zangfx_impl_handle, zangfx_impl_object, Result};
use zangfx_utils::asyncheap;

#[derive(Debug)]
struct Heap(Mutex<HeapData>);

#[derive(Debug)]
struct HeapData {
    size: u64,
    allocated: u64,
}

#[derive(Debug, Clone)]
struct Buffer {
    size: u64,
}

zangfx_impl_handle! { Buffer, base::BufferRef }

unsafe impl base::Buffer for Buffer {
    fn as_ptr(&self) -> *mut u8 {
        unreachable!()
    }

    fn len(&self) -> base::DeviceSize {
        self.size
    }

    fn make_proxy(&self, _queue: &base::CmdQueueRef) -> base::BufferRef {
        unreachable!()
    }

    fn get_memory_req(&self) -> Result<base::MemoryReq> {
        unreachable!()
    }
}

zangfx_impl_object! { Heap: dyn base::Heap, dyn (std::fmt::Debug) }

impl base::Heap for Heap {
    fn bind(&self, obj: base::ResourceRef<'_>) -> Result<bool> {
        let mut data = self.0.lock().unwrap();

        let my_buffer: &Buffer = if let base::ResourceRef::Buffer(buffer) = obj {
            buffer.downcast_ref().unwrap()
        } else {
            unreachable!()
        };

        if data.allocated + my_buffer.size > data.size {
            Ok(false)
        } else {
            data.allocated += my_buffer.size;
            Ok(true)
        }
    }

    fn make_aliasable(&self, obj: base::ResourceRef<'_>) -> Result<()> {
        let mut data = self.0.lock().unwrap();

        let my_buffer: &Buffer = if let base::ResourceRef::Buffer(buffer) = obj {
            buffer.downcast_ref().unwrap()
        } else {
            unreachable!()
        };

        data.allocated -= my_buffer.size;

        Ok(())
    }
}

fn new_heap(size: u64) -> base::HeapRef {
    Arc::new(Heap(Mutex::new(HeapData { size, allocated: 0 })))
}

fn new_buffer(size: u64) -> base::BufferRef {
    Buffer { size }.into()
}

#[test]
fn nonblocking() {
    let heap = new_heap(100);
    let async_heap = asyncheap::AsyncHeap::new(heap);
    let buffer1 = new_buffer(40);
    let buffer2 = new_buffer(40);
    block_on(async_heap.bind((&buffer1).into())).unwrap();
    block_on(async_heap.bind((&buffer2).into())).unwrap();
}

#[test]
fn blocking() {
    let heap = new_heap(100);
    let async_heap = asyncheap::AsyncHeap::new(heap);
    let buffer1 = new_buffer(40);
    let buffer2 = new_buffer(40);
    block_on(async_heap.bind((&buffer1).into())).unwrap();
    block_on(async_heap.bind((&buffer2).into())).unwrap();

    let state = Arc::new(AtomicUsize::new(0));

    // This allocation ...
    let buffer3 = new_buffer(40);
    let bind3 = async_heap.bind((&buffer3).into());

    let state2 = state.clone();
    let join_handle = thread::Builder::new()
        .spawn(move || {
            state2.store(1, Ordering::Relaxed);
            block_on(bind3).unwrap();
        })
        .unwrap();

    while state.load(Ordering::Relaxed) == 0 {
        thread::yield_now();
    }

    thread::yield_now();
    thread::sleep(Duration::from_millis(10));

    // should block because there isn't a room
    assert_eq!(state.load(Ordering::Relaxed), 1);

    // ... But releasing one of the buffers should unblock it
    let make_aliasable1 =
        future::lazy(|_| future::result(async_heap.make_aliasable((&buffer1).into())));

    block_on(make_aliasable1).unwrap();

    join_handle.join().unwrap();
}
