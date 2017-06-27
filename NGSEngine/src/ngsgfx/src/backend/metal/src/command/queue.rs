//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, metal, block, OCPtr};

use std::sync::atomic::Ordering;
use std::mem::forget;

use imp::{Backend, Fence, CommandBuffer};

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct CommandQueue {
    obj: OCPtr<metal::MTLCommandQueue>,
}

unsafe impl Send for CommandQueue {}

impl CommandQueue {
    pub(crate) fn new(obj: metal::MTLCommandQueue) -> Self {
        Self { obj: OCPtr::new(obj).unwrap() }
    }
}

struct SubmissionTransaction<'a> {
    buffers: &'a [&'a CommandBuffer],
    num_successful_transitions: usize,
    fence_associated: Option<&'a Fence>,
}

fn submit_commands(buffers: &[&CommandBuffer], fence: Option<&Fence>) -> core::Result<()> {
    let mut transaction = SubmissionTransaction {
        buffers: buffers,
        num_successful_transitions: 0,
        fence_associated: None,
    };

    // Check some preconditions beforehand
    // (this eases error handling)
    for buffer in buffers.iter() {
        buffer.buffer.as_ref().expect(
            "invalid command buffer state",
        );
        if buffer.encoder.is_recording() {
            panic!("invalid command buffer state");
        }
        // now we are sure this buffer is in the
        // `Executable`, `Pending`, or `Completed`
    }

    let num_buffers = buffers.len();

    // Make a state transition from `Executable` to `Pending`
    'check_state: for buffer in buffers.iter() {
        let ov = buffer.submitted.swap(true, Ordering::Acquire);
        if ov {
            // Some buffers were not in `Executable`;
            panic!("invalid command buffer state");
        }
        transaction.num_successful_transitions += 1;
    }

    let mut completion_handler = None;

    // Prepare fence
    if let Some(fence) = fence {
        let result = fence.associate_pending_buffers(num_buffers);
        transaction.fence_associated = Some(fence);

        // The fence must be unsignalled
        assert!(result, "fence must be in the unsignalled state");

        let fence_ref: Fence = fence.clone();
        let block = block::ConcreteBlock::new(move |_| { fence_ref.remove_pending_buffers(1); });
        completion_handler = Some(block.copy());
    }

    for buffer in buffers.iter() {
        let metal_buffer = buffer.buffer.as_ref().unwrap();
        if let Some(ref completion_handler) = completion_handler {
            metal_buffer.add_completed_handler(&**completion_handler);
        }

        metal_buffer.commit();
    }

    // The operation was successful; now commit the transaction
    forget(transaction);

    Ok(())
}

impl<'a> Drop for SubmissionTransaction<'a> {
    fn drop(&mut self) {
        // Perform rollback
        for buffer in self.buffers.iter() {
            if self.num_successful_transitions == 0 {
                break;
            }
            self.num_successful_transitions -= 1;
            buffer.submitted.store(false, Ordering::Release);
        }

        if let Some(fence) = self.fence_associated {
            fence.remove_pending_buffers(self.num_successful_transitions);
        }
    }
}

impl core::Marker for CommandQueue {
    fn set_label(&self, label: Option<&str>) {
        self.obj.set_label(label.unwrap_or(""));
    }
}

impl core::CommandQueue<Backend> for CommandQueue {
    fn make_command_buffer(&self) -> core::Result<CommandBuffer> {
        Ok(CommandBuffer::new(*self.obj))
    }

    fn wait_idle(&self) {
        unimplemented!()
    }

    fn submit_commands(
        &self,
        buffers: &[&CommandBuffer],
        fence: Option<&Fence>,
    ) -> core::Result<()> {
        submit_commands(buffers, fence)
    }
}
