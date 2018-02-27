//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Implementation of `CmdBuffer` for Metal.
use std::fmt;
use std::mem::replace;
use parking_lot::Mutex;

use base::command;
use common::Result;
use metal::MTLCommandBuffer;
use utils::OCPtr;

use super::enc::CmdBufferFenceSet;
use super::enc_compute::ComputeEncoder;

/// Implementation of `CmdBuffer` for Metal.
pub struct CmdBuffer {
    metal_buffer: OCPtr<MTLCommandBuffer>,
    completion_callbacks: Vec<Box<FnMut()>>,
    fence_set: CmdBufferFenceSet,

    /// Currently active encoder.
    encoder: Option<Encoder>,
}

zangfx_impl_object! { CmdBuffer: command::CmdBuffer, ::Debug }

#[derive(Debug)]
enum Encoder {
    Compute(ComputeEncoder),
}

unsafe impl Send for CmdBuffer {}
unsafe impl Sync for CmdBuffer {}

impl CmdBuffer {
    pub unsafe fn from_raw(metal_buffer: MTLCommandBuffer) -> Self {
        Self {
            metal_buffer: OCPtr::from_raw(metal_buffer).unwrap(),
            completion_callbacks: Vec::new(),
            fence_set: CmdBufferFenceSet::new(),
            encoder: None,
        }
    }

    /// Clear `self.encoder` and take `fence_set` back from it
    fn clear_encoder(&mut self) {
        if let Some(enc) = self.encoder.take() {
            match enc {
                Encoder::Compute(e) => self.fence_set = e.finish(),
            }
        }
    }
}

impl fmt::Debug for CmdBuffer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("CmdBuffer")
            .field("metal_buffer", &self.metal_buffer)
            .field(
                "completion_callbacks",
                &format!("[{} elements]", self.completion_callbacks.len()),
            )
            .finish()
    }
}

impl command::CmdBuffer for CmdBuffer {
    fn enqueue(&mut self) -> Result<()> {
        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        use block;
        use std::mem::replace;

        self.clear_encoder();

        let callbacks = replace(&mut self.completion_callbacks, vec![]);
        if callbacks.len() > 0 {
            let callbacks_cell = Mutex::new(callbacks);
            let block = block::ConcreteBlock::new(move |_| {
                for cb in callbacks_cell.lock().iter_mut() {
                    cb();
                }
            });
            self.metal_buffer.add_completed_handler(&block.copy());
        }

        unimplemented!();
    }

    fn encode_render(&mut self) -> &mut command::RenderCmdEncoder {
        unimplemented!();
    }
    fn encode_compute(&mut self) -> &mut command::ComputeCmdEncoder {
        self.clear_encoder();

        let metal_encoder = self.metal_buffer.new_compute_command_encoder();
        // TODO: handle nil `metal_encoder`

        // Create a `ComputeEncoder` and move `self.fence_set` to it
        let encoder = unsafe {
            ComputeEncoder::new(
                metal_encoder,
                replace(&mut self.fence_set, Default::default()),
            )
        };
        self.encoder = Some(Encoder::Compute(encoder));
        match self.encoder {
            Some(Encoder::Compute(ref mut e)) => e,
            _ => unreachable!(),
        }
    }
    fn encode_copy(&mut self) -> &mut command::CopyCmdEncoder {
        unimplemented!();
    }

    fn on_complete(&mut self, cb: Box<FnMut()>) {
        self.completion_callbacks.push(cb);
    }
}
