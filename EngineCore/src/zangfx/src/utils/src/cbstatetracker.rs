//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tracks the execution state of command buffers.
use std::sync::Arc;
use std::sync::{Condvar, Mutex};
use std::time::Duration;
use zangfx_base as base;

/// Tracks the execution state of a command buffer.
#[derive(Debug)]
pub struct CbStateTracker {
    state: Arc<State>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaitTimeoutError {
    Timeout,
}

#[derive(Debug)]
struct State {
    done: Mutex<Option<base::Result<()>>>,
    cv: Condvar,
}

impl CbStateTracker {
    pub fn new(cmd_buffer: &mut dyn base::CmdBuffer) -> Self {
        let state = Arc::new(State {
            done: Mutex::new(None),
            cv: Condvar::new(),
        });
        {
            let state = Arc::clone(&state);
            cmd_buffer.on_complete(Box::new(move |result| {
                let mut done = state.done.lock().unwrap();
                *done = Some(result);
                state.cv.notify_all();
            }));
        }
        Self { state }
    }

    pub fn is_completed(&self) -> bool {
        self.state.done.lock().unwrap().is_some()
    }

    pub fn wait(&self) -> &base::Result<()> {
        let mut done = self.state.done.lock().unwrap();
        while done.is_none() {
            done = self.state.cv.wait(done).unwrap();
        }

        // Extend the lifetime - it is safe since nobody will mutate it afterwards
        let result_ref = done.as_ref().unwrap();
        unsafe { &*(result_ref as *const _) }
    }

    pub fn wait_timeout(&self, timeout: Duration) -> Result<&base::Result<()>, WaitTimeoutError> {
        let mut done = self.state.done.lock().unwrap();
        while done.is_none() {
            // FIXME: This might block longer than intended in some cases
            let (new_guard, result) = self.state.cv.wait_timeout(done, timeout).unwrap();
            done = new_guard;
            if result.timed_out() {
                return Err(WaitTimeoutError::Timeout);
            }
        }

        // Extend the lifetime - it is safe since nobody will mutate it afterwards
        let result_ref = done.as_ref().unwrap();
        Ok(unsafe { &*(result_ref as *const _) })
    }
}
