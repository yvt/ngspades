//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdBuffer` for Metal.
use std::fmt;
use std::sync::Arc;
use std::mem::replace;
use parking_lot::Mutex;

use base::command;
use common::{Error, ErrorKind, Result};
use metal::{MTLCommandBuffer, MTLCommandQueue};
use utils::{nil_error, OCPtr};

use super::queue::{CommitedBuffer, Scheduler};
use super::enc::CmdBufferFenceSet;
use super::enc_compute::ComputeEncoder;

/// Implementation of `CmdBuffer` for Metal.
#[derive(Debug)]
pub struct CmdBuffer {
    /// `uncommited.is_some()` iff the command buffer is not commited yet.
    uncommited: Option<UncommitedBuffer>,

    /// The queue scheduler.
    scheduler: Arc<Scheduler>,
}

zangfx_impl_object! { CmdBuffer: command::CmdBuffer, ::Debug }

#[derive(Debug)]
struct UncommitedBuffer {
    metal_buffer: OCPtr<MTLCommandBuffer>,
    fence_set: CmdBufferFenceSet,

    /// The set of registered completion callbacks. Passed to `MTLCommandBuffer`
    /// on commit.
    completion_callbacks: CallbackSet,

    /// Currently active encoder.
    encoder: Option<Encoder>,
}

#[derive(Default)]
struct CallbackSet(Vec<Box<FnMut()>>);

#[derive(Debug)]
enum Encoder {
    Compute(ComputeEncoder),
}

unsafe impl Send for CmdBuffer {}
unsafe impl Sync for CmdBuffer {}

fn already_commited_error() -> Error {
    Error::with_detail(
        ErrorKind::InvalidUsage,
        "command buffer is already commited",
    )
}

impl CmdBuffer {
    pub(super) unsafe fn new(
        metal_queue: MTLCommandQueue,
        scheduler: Arc<Scheduler>,
    ) -> Result<Self> {
        let metal_buffer = metal_queue.new_command_buffer();
        if metal_buffer.is_null() {
            return Err(nil_error("MTLCommandQueue newCommandBuffer"));
        }

        Ok(Self {
            uncommited: Some(UncommitedBuffer {
                metal_buffer: OCPtr::new(metal_buffer).unwrap(),
                completion_callbacks: Default::default(),
                fence_set: CmdBufferFenceSet::new(),
                encoder: None,
            }),
            scheduler,
        })
    }
}

impl fmt::Debug for CallbackSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("CallbackSet")
            .field(&format!("[{} elements]", self.0.len()))
            .finish()
    }
}

impl UncommitedBuffer {
    /// Clear `self.encoder` and take `fence_set` back from it.
    fn clear_encoder(&mut self) {
        if let Some(enc) = self.encoder.take() {
            match enc {
                Encoder::Compute(e) => self.fence_set = e.finish(),
            }
        }
    }
}

impl command::CmdBuffer for CmdBuffer {
    fn enqueue(&mut self) -> Result<()> {
        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        use block;
        use std::mem::replace;

        // Commiting a command buffer implicitly enqueues it
        self.enqueue()?;

        let mut uncommited = self.uncommited.take().ok_or_else(already_commited_error)?;
        uncommited.clear_encoder();

        // Pass the completion callbacks to `MTLCommandBuffer`
        let callbacks = replace(&mut uncommited.completion_callbacks, Default::default());
        if callbacks.0.len() > 0 {
            let callbacks_cell = Mutex::new(callbacks.0);
            let block = block::ConcreteBlock::new(move |_| {
                for cb in callbacks_cell.lock().iter_mut() {
                    cb();
                }
            });
            uncommited.metal_buffer.add_completed_handler(&block.copy());
        }

        // Commit the command buffer
        self.scheduler.commit(CommitedBuffer {
            metal_buffer: uncommited.metal_buffer,
            fence_set: uncommited.fence_set,
        });

        Ok(())
    }

    fn encode_render(&mut self) -> &mut command::RenderCmdEncoder {
        unimplemented!();
    }
    fn encode_compute(&mut self) -> &mut command::ComputeCmdEncoder {
        let uncommited = self.uncommited
            .as_mut()
            .ok_or_else(already_commited_error)
            .unwrap();
        uncommited.clear_encoder();

        let metal_encoder = uncommited.metal_buffer.new_compute_command_encoder();
        // TODO: handle nil `metal_encoder`

        // Create a `ComputeEncoder` and move `uncommited.fence_set` to it
        let encoder = unsafe {
            ComputeEncoder::new(
                metal_encoder,
                replace(&mut uncommited.fence_set, Default::default()),
            )
        };
        uncommited.encoder = Some(Encoder::Compute(encoder));
        match uncommited.encoder {
            Some(Encoder::Compute(ref mut e)) => e,
            _ => unreachable!(),
        }
    }
    fn encode_copy(&mut self) -> &mut command::CopyCmdEncoder {
        unimplemented!();
    }

    fn on_complete(&mut self, cb: Box<FnMut()>) {
        let uncommited = self.uncommited
            .as_mut()
            .ok_or_else(already_commited_error)
            .unwrap();
        uncommited.completion_callbacks.0.push(cb);
    }
}
