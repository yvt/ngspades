//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdBuffer` for Metal.
use atomic_refcell::AtomicRefCell;
use std::fmt;
use std::mem::replace;
use std::sync::Arc;
use zangfx_metal_rs::{MTLCommandBuffer, MTLCommandBufferStatus, MTLCommandQueue};

use crate::renderpass::RenderTargetTable;
use crate::utils::{nil_error, OCPtr};
use zangfx_base::{self as base, command};
use zangfx_base::{interfaces, vtable_for, zangfx_impl_object};
use zangfx_base::{Error, ErrorKind, Result};

use super::enc::CmdBufferFenceSet;
use super::enc_compute::ComputeEncoder;
use super::enc_copy::CopyEncoder;
use super::enc_render::RenderEncoder;
use super::queue::{CommitedBuffer, Scheduler};

/// Implementation of `CmdBuffer` for Metal.
#[derive(Debug)]
pub struct CmdBuffer {
    /// `uncommited.is_some()` iff the command buffer is not commited yet.
    uncommited: Option<UncommitedBuffer>,

    /// The queue scheduler.
    scheduler: Arc<Scheduler>,
}

zangfx_impl_object! { CmdBuffer: dyn command::CmdBuffer, dyn crate::Debug, dyn base::SetLabel }

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
struct CallbackSet(Vec<Box<dyn FnMut(Result<()>) + Sync + Send>>);

#[derive(Debug)]
enum Encoder {
    Compute(ComputeEncoder),
    Render(RenderEncoder),
    Copy(CopyEncoder),
}

unsafe impl Send for CmdBuffer {}
unsafe impl Sync for CmdBuffer {}

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

    /// Return the underlying `MTLCommandBuffer` object. Returns `None` if the
    /// command buffer is already committed.
    pub fn metal_cmd_buffer(&self) -> Option<MTLCommandBuffer> {
        self.uncommited.as_ref().map(|c| *c.metal_buffer)
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
    fn commit(&mut self) -> Result<()> {
        use block;
        use std::mem::replace;

        let mut uncommited = self
            .uncommited
            .take()
            .expect("command buffer is already commited");
        uncommited.clear_encoder();

        // Pass the completion callbacks to `MTLCommandBuffer`
        let callbacks = replace(&mut uncommited.completion_callbacks, Default::default());
        if callbacks.0.len() > 0 {
            let callbacks_cell = AtomicRefCell::new(callbacks.0);
            let metal_buffer = Clone::clone(&uncommited.metal_buffer);
            let block = block::ConcreteBlock::new(move |_| {
                // TODO: Return error details (`MTLCommandBufferError`?)

                // `Error` is not `Clone`, so it must be re-created for every
                // iteration.
                let status = metal_buffer.status();
                for cb in callbacks_cell.borrow_mut().iter_mut() {
                    cb(match status {
                        MTLCommandBufferStatus::Completed => Ok(()),
                        _ => Err(Error::new(ErrorKind::Other)),
                    });
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
        render_target_table: &base::RenderTargetTableRef,
    ) -> &mut dyn command::RenderCmdEncoder {
        let our_rt_table: &RenderTargetTable = render_target_table
            .downcast_ref()
            .expect("bad render target table type");

        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
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
                our_rt_table.extents(),
            )
        };
        uncommited.encoder = Some(Encoder::Render(encoder));
        match uncommited.encoder {
            Some(Encoder::Render(ref mut e)) => e,
            _ => unreachable!(),
        }
    }
    fn encode_compute(&mut self) -> &mut dyn command::ComputeCmdEncoder {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
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
    fn encode_copy(&mut self) -> &mut dyn command::CopyCmdEncoder {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
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

    fn on_complete(&mut self, cb: Box<dyn FnMut(Result<()>) + Sync + Send>) {
        let uncommited = self
            .uncommited
            .as_mut()
            .expect("command buffer is already commited");
        uncommited.completion_callbacks.0.push(cb);
    }
}
