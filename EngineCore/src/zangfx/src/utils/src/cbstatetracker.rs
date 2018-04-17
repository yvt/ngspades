//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tracks the execution state of command buffers.
use std::sync::Arc;
use std::sync::{Condvar, Mutex};
use std::time::Duration;
use base;

/// Tracks the execution state of a command buffer.
#[derive(Debug)]
pub struct CbStateTracker {
    state: Arc<State>,
}

#[derive(Debug)]
struct State {
    done: Mutex<bool>,
    cv: Condvar,
}

impl CbStateTracker {
    pub fn new(cmd_buffer: &mut base::CmdBuffer) -> Self {
        let state = Arc::new(State {
            done: Mutex::new(false),
            cv: Condvar::new(),
        });
        {
            let state = Arc::clone(&state);
            cmd_buffer.on_complete(Box::new(move || {
                let mut done = state.done.lock().unwrap();
                *done = true;
                state.cv.notify_all();
            }));
        }
        Self { state }
    }

    pub fn is_completed(&self) -> bool {
        *self.state.done.lock().unwrap()
    }

    pub fn wait(&self) {
        let mut done = self.state.done.lock().unwrap();
        while !*done {
            done = self.state.cv.wait(done).unwrap();
        }
    }

    pub fn wait_timeout(&self, timeout: Duration) -> Result<(), ()> {
        let mut done = self.state.done.lock().unwrap();
        while !*done {
            // FIXME: This might block longer than intended in some cases
            let (new_guard, result) = self.state.cv.wait_timeout(done, timeout).unwrap();
            done = new_guard;
            if result.timed_out() {
                return Err(());
            }
        }
        Ok(())
    }
}
