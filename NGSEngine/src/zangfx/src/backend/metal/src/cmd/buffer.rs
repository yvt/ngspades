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
use metal::{MTLCommandBuffer, MTLCommandQueue};

use base::{self, command, handles};
use common::{Error, ErrorKind, Result};
use utils::{nil_error, OCPtr};
use renderpass::RenderTargetTable;

use super::queue::{CommitedBuffer, Scheduler};
use super::enc::CmdBufferFenceSet;
use super::enc_compute::ComputeEncoder;
use super::enc_copy::CopyEncoder;
use super::enc_render::RenderEncoder;

/// Implementation of `CmdBuffer` for Metal.
#[derive(Debug)]
pub struct CmdBuffer {
    /// `uncommited.is_some()` iff the command buffer is not commited yet.
    uncommited: Option<UncommitedBuffer>,

    /// The queue scheduler.
    scheduler: Arc<Scheduler>,
}

zangfx_impl_object! { CmdBuffer: command::CmdBuffer, ::Debug, base::SetLabel }

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
struct CallbackSet(Vec<Box<FnMut() + Sync + Send>>);

#[derive(Debug)]
enum Encoder {
    Compute(ComputeEncoder),
    Render(RenderEncoder),
    Copy(CopyEncoder),
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
                Encoder::Render(e) => self.fence_set = e.finish(),
                Encoder::Copy(e) => self.fence_set = e.finish(),
            }
        }
    }
}

impl Drop for CmdBuffer {
    fn drop(&mut self) {
        // We always must call `[MTLCommandEncoder endEncoding]` before
        // deallocating it. (Otherwise an assertion failure would occur)
        if let Some(ref mut uncommited) = self.uncommited {
            uncommited.clear_encoder();
        }
    }
}

impl base::SetLabel for CmdBuffer {
    fn set_label(&mut self, label: &str) {
        if let Some(ref mut uncommited) = self.uncommited {
            uncommited.metal_buffer.set_label(label);
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

    fn encode_render(
        &mut self,
        render_target_table: &handles::RenderTargetTable,
    ) -> &mut command::RenderCmdEncoder {
        let our_rt_table: &RenderTargetTable = render_target_table
            .downcast_ref()
            .expect("bad render target table type");

        let uncommited = self.uncommited
            .as_mut()
            .ok_or_else(already_commited_error)
            .unwrap();
        uncommited.clear_encoder();

        let metal_encoder = uncommited
            .metal_buffer
            .new_render_command_encoder(our_rt_table.metal_render_pass());
        // TODO: handle nil `metal_encoder`

        // Create a `RenderEncoder` and move `uncommited.fence_set` to it
        let encoder = unsafe {
            RenderEncoder::new(
                metal_encoder,
                replace(&mut uncommited.fence_set, Default::default()),
            )
        };
        uncommited.encoder = Some(Encoder::Render(encoder));
        match uncommited.encoder {
            Some(Encoder::Render(ref mut e)) => e,
            _ => unreachable!(),
        }
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
        let uncommited = self.uncommited
            .as_mut()
            .ok_or_else(already_commited_error)
            .unwrap();
        uncommited.clear_encoder();

        let metal_encoder = uncommited.metal_buffer.new_blit_command_encoder();
        // TODO: handle nil `metal_encoder`

        // Create a `CopyEncoder` and move `uncommited.fence_set` to it
        let encoder = unsafe {
            CopyEncoder::new(
                metal_encoder,
                replace(&mut uncommited.fence_set, Default::default()),
            )
        };
        uncommited.encoder = Some(Encoder::Copy(encoder));
        match uncommited.encoder {
            Some(Encoder::Copy(ref mut e)) => e,
            _ => unreachable!(),
        }
    }

    fn on_complete(&mut self, cb: Box<FnMut() + Sync + Send>) {
        let uncommited = self.uncommited
            .as_mut()
            .ok_or_else(already_commited_error)
            .unwrap();
        uncommited.completion_callbacks.0.push(cb);
    }
}
