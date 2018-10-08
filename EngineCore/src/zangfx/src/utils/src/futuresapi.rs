//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a `Future`-based interface.
use futures::channel::oneshot;

use zangfx_base::{CmdBuffer, Result};

/// A type implementing
/// `Future<zangfx::base::Result<()>, CmdBufferCanceled> + Sync + Send` that
/// represents a result of a command buffer execution.
pub type CmdBufferResult = oneshot::Receiver<Result<()>>;

/// Error returned from a `CmdBufferResult` when a command buffer execution was
/// canceled.
pub type CmdBufferCanceled = oneshot::Canceled;

/// Provides a `Future`-based interface for `CmdBuffer`.
pub trait CmdBufferFutureExt: CmdBuffer {
    /// Construct a `Future` representing the result of the execution of this
    /// command buffer.
    ///
    /// This method is implemented using `CmdBuffer::on_complete`, so the valid
    /// usages of that method must be obeyed.
    fn result(&mut self) -> CmdBufferResult;
}

impl<T: ?Sized + CmdBuffer> CmdBufferFutureExt for T {
    fn result(&mut self) -> CmdBufferResult {
        let (sender, receiver) = oneshot::channel();

        let mut sender_cell = Some(sender);

        self.on_complete(Box::new(move |result| {
            let sender = sender_cell.take().unwrap();

            // Don't care even if the receiving end has been already closed
            let _ = sender.send(result);
        }));

        receiver
    }
}
