//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Collects abandoned objects in a background thread.
use std::thread;
use std::sync::{mpsc, Arc};
use std::mem;
use ngsgfx_common::atom2::AtomicArc;
use parking_lot::Mutex;

use DeviceRef;
use super::LlFence;

#[derive(Debug)]
pub(super) struct Recycler<T> {
    thread: Option<thread::JoinHandle<()>>,
    sender: Mutex<mpsc::Sender<T>>,
}

impl<T> Drop for Recycler<T> {
    fn drop(&mut self) {
        self.thread.take().unwrap().join().unwrap();
    }
}

impl<T: Send + 'static> Recycler<T> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .spawn(move || while let Ok(obj) = rx.recv() {})
            .unwrap();
        Self {
            thread: Some(thread),
            sender: Mutex::new(tx),
        }
    }

    pub fn recycle(&self, obj: T) {
        self.sender.lock().send(obj).unwrap();
    }
}
