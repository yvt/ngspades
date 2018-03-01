//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::sync::mpsc;
use std::time::Duration;

use base;

#[derive(Debug)]
pub struct CmdBufferAwaiter {
    recv: mpsc::Receiver<()>,
}

impl CmdBufferAwaiter {
    pub fn new(buffer: &mut base::command::CmdBuffer) -> Self {
        let (send, recv) = mpsc::channel();

        buffer.on_complete(Box::new(move || {
            let _ = send.send(());
        }));

        Self { recv }
    }

    pub fn wait_until_completed(&self) {
        self.recv.recv_timeout(Duration::from_millis(1000)).unwrap();
    }
}
