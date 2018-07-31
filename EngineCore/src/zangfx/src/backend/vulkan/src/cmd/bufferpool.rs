//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::version::*;
use ash::vk;
use parking_lot::Mutex;
use std::mem::ManuallyDrop;
use std::ops;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;

use zangfx_base::Result;

use crate::device::DeviceRef;
use crate::utils::translate_generic_error_unwrap;

use super::buffer::CmdBufferData;

/// A thread-safe pool type that maintains a fixed number of items.
#[derive(Debug)]
crate struct CbPool<T: CbPoolContent> {
    data: Mutex<PoolData<T>>,
    send: SyncSender<T>,
}

/// Non-`Sync` data.
#[derive(Debug)]
struct PoolData<T: CbPoolContent> {
    recv: Receiver<T>,
}

/// An item allocated from `CbPool`. Returned to the original
/// pool on drop.
#[derive(Debug)]
crate struct CbPoolItem<T: CbPoolContent> {
    payload: ManuallyDrop<T>,
    send: SyncSender<T>,
}

crate trait CbPoolContent {
    fn reset(&mut self);
}

impl<T: CbPoolContent> CbPool<T> {
    crate fn new<I>(mut items: I) -> Result<Self>
    where
        I: Iterator<Item = Result<T>> + ExactSizeIterator,
    {
        let len = items.len();
        let (send, recv) = sync_channel(len);
        for item in items {
            send.send(item?).unwrap();
        }

        Ok(Self {
            data: Mutex::new(PoolData { recv }),
            send,
        })
    }

    /// Allocate an empty item. Might block if there are an excessive
    /// number of outstanding allocated items.
    crate fn allocate(&self) -> CbPoolItem<T> {
        use std::mem::drop;

        let send = self.send.clone();

        let data = self.data.lock();
        let payload = ManuallyDrop::new(data.recv.recv().unwrap());

        CbPoolItem { payload, send }
    }
}

impl<T: CbPoolContent> ops::Deref for CbPoolItem<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.payload
    }
}

impl<T: CbPoolContent> ops::DerefMut for CbPoolItem<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.payload
    }
}

impl<T: CbPoolContent> Drop for CbPoolItem<T> {
    fn drop(&mut self) {
        use std::ptr::read;

        // Move out the payload
        let mut payload = unsafe { read(&*self.payload) };

        payload.reset();

        // Return the command buffer to the pool. Do not care even if `send`
        // fails, in which case `CbPool` already have released the
        // pool as well as all command buffers.
        let _ = self.send.send(payload);
    }
}

impl<T: CbPoolContent + ?Sized> CbPoolContent for Box<T> {
    fn reset(&mut self) {
        (**self).reset()
    }
}
