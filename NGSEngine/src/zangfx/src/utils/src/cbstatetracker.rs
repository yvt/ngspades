//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tracks the execution state of command buffers.
use std::sync::mpsc;
use std::time::Duration;
use base;

/// Tracks the execution state of a command buffer.
#[derive(Debug)]
pub struct CbStateTracker {
    recv: mpsc::Receiver<()>,
}

impl CbStateTracker {
    pub fn new(cmd_buffer: &mut base::CmdBuffer) -> Self {
        let (send, recv) = mpsc::sync_channel(0);
        cmd_buffer.on_complete(Box::new(move || {
            let _ = send;
        }));
        Self { recv }
    }

    pub fn is_completed(&self) -> bool {
        self.recv.try_recv().is_err()
    }

    pub fn wait(&self) {
        // This will unblock once `mpsc::Sender` is dropped
        let _ = self.recv.recv();
    }

    pub fn wait_timeout(&self, timeout: Duration) -> Result<(), ()> {
        match self.recv.recv_timeout(timeout) {
            Ok(()) => unreachable!(),
            Err(mpsc::RecvTimeoutError::Disconnected) => Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(()),
        }
    }
}
