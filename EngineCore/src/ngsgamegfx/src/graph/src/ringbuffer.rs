//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! This module implements a ring buffer. It provides `taskman`-based tasks that
//! allocate memory regions in a hypothetical byte-addressed memory space based
//! on the clients' requests and return their byte offsets.
//! The actual backing store must be supplied to clients via other means.
//!
//! The usage pattern is shown below:
//!
//!  1. **Clients** calculate the number of bytes they want to allocate in a
//!     ring buffer and creates allocation requests. Each of the requests is
//!     passed to **the ring buffer manager** via a cell.
//!  2. The manager collects and processes all requests. If no sufficient free
//!     space is available in the ring buffer, the manager blocks until all
//!     requests can be processed.
//!  3. After memory regions are allocated for all requests, the current
//!     command buffer is associated with the set of allocated memory regions.
//!     The manager makes sure that regions remain valid until the execution of
//!     the command buffer is complete.
//!  4. The clients receive a byte range corresponding to each allocation
//!     request. A buffer object itself must be supplied via other means.
//!     The clients would write data to be read by a GPU on the buffer object.
//!  5. The buffer object is referenced by an encoded command buffer via an
//!     argument table or other means. Note that this can be done independently
//!     from 4.
//!  6. Finally, the command buffer is submitted. This must be done after both
//!     of 4 and 5.
//!
//! ```text
//!                                 ,--------------,
//!                                 | CB allocator |
//! ,--- Client 1 ---,              '--------------'     ,--- Client 2 ---,
//! |                | 1.  ,-------------,     |    1.   |                |
//! |  Compute size -----> | Ring buffer | <---)----------- Compute size  |
//! |                |     |   manager   |     |         |                |
//! |           4.   |     '------+------'     |         |   4.           |
//! | Fill contents <-----o-------^------------)-----o----> Fill contents |
//! | |         5.   |    ↓                    |     ↓   |   5.         | |
//! | | CB encoder <---------------------------^-----------> CB encoder | |
//! | |      |       |                                   |        |     | |
//! '-|------|-------'                                   '--------|-----|-'
//!   '----->o                  6.  ,---------------, 6.          o<----'
//!          '--------------------> | CB dispatcher | <-----------'
//!                                 '---------------'
//!                       ,--------------,    | 3.
//!                       | Store Future | <--'
//!                       '--------------'
//! ```
//!
use arrayvec::ArrayVec;
use futures::{channel::oneshot, executor::block_on, future::ready, prelude::*, Future};
use std::{ops::Range, pin::Pin};
use zangfx::{base as gfx, utils::CmdBufferResult};

use crate::taskman::{CellRef, GraphBuilder, GraphContext, Task, TaskInfo};
use ngsgamegfx_common::asyncring;

/// A type used to create `taskman`-based tasks for maintaining a ring buffer
/// on a byte-addressed buffer.
#[derive(Debug)]
pub struct RingBufferBuilder {
    clients: Vec<Client>,
}

/// An allocation request.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct AllocReq {
    pub size: u32,
    pub align: u32,
}

/// Represents a range in a buffer.
pub type Alloc = Range<u64>;

/// A set of cells for passing data between the manager and client tasks.
/// Created by `RingBufferBuilder::define_client`.
pub type Client = (CellRef<Vec<AllocReq>>, CellRef<Vec<Alloc>>);

/// The specialized version of `AsyncRing` for `ringbuffer`.
pub type AsyncRing<E> =
    asyncring::AsyncRing<Pin<Box<Future<Output = Result<(), E>> + Send + Sync>>, u32>;

impl RingBufferBuilder {
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    /// Define a client.
    ///
    /// This method returns two cells:
    ///
    ///  - The first cell is used to pass allocation requests from a client's
    ///    task to the manager.
    ///  - The second cell is used to pass allocation information from the
    ///    manager to a client's other tasks.
    ///
    pub fn define_client<T>(&mut self, graph_builder: &mut GraphBuilder<T>) -> Client {
        let client = (
            graph_builder.define_cell(Vec::new()),
            graph_builder.define_cell(Vec::new()),
        );
        self.clients.push(client);
        client
    }

    /// Register tasks to `GraphBuilder`, consuming `self`.
    ///
    /// See [`RingBufferBuilder::add_to_graph_inner`] for details.
    pub fn add_to_graph(
        self,
        graph_builder: &mut GraphBuilder<gfx::Error>,
        cmd_buffer_result: CellRef<Option<CmdBufferResult>>,
        async_ring: AsyncRing<gfx::Error>,
    ) {
        self.add_to_graph_inner(graph_builder, cmd_buffer_result, async_ring, |r| {
            r.expect("command buffer was unexpectedly cancelled")
        })
    }

    /// Register tasks to `GraphBuilder`, consuming `self`.
    ///
    /// `transform_result` is a closure used to convert the output of the
    /// `Future` retrieved via `cmd_buffer_result` (`R::Output`) into
    /// `Result<(), E>`.
    ///
    /// Note: `cmd_buffer_result` is only polled when waited by the acquire
    /// task.
    pub fn add_to_graph_inner<R, E>(
        self,
        graph_builder: &mut GraphBuilder<E>,
        cmd_buffer_result: CellRef<Option<R>>,
        async_ring: AsyncRing<E>,
        transform_result: impl Fn(R::Output) -> Result<(), E> + Send + Sync + Clone + 'static,
    ) where
        R: Future + std::fmt::Debug + Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        let async_ring = graph_builder.define_cell(WrapDebug(async_ring));
        let cmd_buffer_result_sender = graph_builder.define_cell(None);

        let cell_uses: Vec<_> = (self.clients.iter())
            .map(|(in_reqs, out_alloc)| {
                ArrayVec::from([in_reqs.use_as_consumer(), out_alloc.use_as_producer()])
            })
            .flatten()
            .chain(ArrayVec::from([
                async_ring.use_as_producer(),
                cmd_buffer_result_sender.use_as_producer(),
            ]))
            .collect();

        graph_builder.define_task(TaskInfo {
            task: Box::new(AllocateTask {
                async_ring,
                cmd_buffer_result_sender,
                transform_result,
                clients: self.clients,
            }),
            cell_uses,
        });

        graph_builder.define_task(TaskInfo {
            task: Box::new(PostSubmissionTask {
                cmd_buffer_result_sender,
                input_cmd_buffer_result: cmd_buffer_result,
            }),
            cell_uses: vec![
                cmd_buffer_result.use_as_consumer(),
                cmd_buffer_result_sender.use_as_consumer(),
            ],
        });
    }
}

struct WrapDebug<T>(T);

impl<T> std::fmt::Debug for WrapDebug<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("WrapDebug").finish()
    }
}

struct AllocateTask<R, E, T> {
    async_ring: CellRef<WrapDebug<AsyncRing<E>>>,
    cmd_buffer_result_sender: CellRef<Option<oneshot::Sender<R>>>,
    transform_result: T,
    clients: Vec<Client>,
}

impl<R, E, T> std::fmt::Debug for AllocateTask<R, E, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("AllocateTask")
            .field("async_ring", &self.async_ring)
            .field("cmd_buffer_result_sender", &self.cmd_buffer_result_sender)
            .field("transform_result", &())
            .field("clients", &self.clients)
            .finish()
    }
}

impl<R, E, T> Task<E> for AllocateTask<R, E, T>
where
    R: Future + Send + Sync + 'static,
    T: Fn(R::Output) -> Result<(), E> + Send + Sync + Clone + 'static,
    E: Send + Sync + 'static,
{
    fn execute(&self, graph_context: &GraphContext) -> Result<(), E> {
        let mut async_ring = graph_context.borrow_cell_mut(self.async_ring);

        // Create a oneshot channel for passing a `Future` from
        // `PostSubmissionTask`.
        //
        // `CmdBufferResult` (a `Future` representing command buffer execution)
        // is created only after command buffer submission, so the
        // `CmdBufferResult` object corresponding to the current submission is
        // not available at this point. However, `AsyncRing` wants a `Future`
        // now.
        //
        // To resolve this, we create a one-shot channel. `PostSubmissionTask`
        // sends a `CmdBufferResult` through this channel. The receiving end is
        // registered to `AsyncRing`.
        let (send, recv) = oneshot::channel();

        // Give the sending end to `PostSubmissionTask`
        *graph_context.borrow_cell_mut(self.cmd_buffer_result_sender) = Some(send);

        // Do allocation
        {
            let mut alloc_back = async_ring.0.alloc_back_multi();

            for (in_req, out_alloc) in self.clients.iter() {
                let in_req = graph_context.borrow_cell(*in_req);
                let mut out_alloc = graph_context.borrow_cell_mut(*out_alloc);

                out_alloc.clear();
                for req in in_req.iter() {
                    let result = block_on(alloc_back.alloc_back_aligned(req.size, req.align))?;

                    // FIXME: fail gracefully
                    let offset = result.expect("ring buffer has been exhausted");

                    out_alloc.push(offset as u64..(offset + req.size) as u64);
                }
            }

            let transform_result = self.transform_result.clone();
            alloc_back.finish(Box::pin(recv.then(|x| {
                // FIXME: Replace this with `either` when ready
                if let Ok(x) = x {
                    Box::pin(x.map(transform_result))
                } else {
                    // The sending end was dropped - probably the last run
                    // did not complete
                    Box::pin(ready(Ok(())))
                        as Pin<Box<dyn Future<Output = Result<(), E>> + Send + Sync>>
                }
            })));
        }

        Ok(())
    }
}

#[derive(Debug)]
struct PostSubmissionTask<R> {
    cmd_buffer_result_sender: CellRef<Option<oneshot::Sender<R>>>,
    input_cmd_buffer_result: CellRef<Option<R>>,
}

impl<R, E> Task<E> for PostSubmissionTask<R>
where
    R: std::fmt::Debug + 'static,
{
    fn execute(&self, graph_context: &GraphContext) -> Result<(), E> {
        let cmd_buffer_result = graph_context
            .borrow_cell_mut(self.input_cmd_buffer_result)
            .take()
            .unwrap();
        let sender = graph_context
            .borrow_cell_mut(self.cmd_buffer_result_sender)
            .take()
            .unwrap();

        let _ = sender.send(cmd_buffer_result);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::taskman::task_from_closure;
    use futures::channel::oneshot;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn no_simultaneous_uses() {
        let mut graph_builder = GraphBuilder::new();

        // Each element represents the state of an object.
        // Zero = free, non-zero = in use
        type State = Vec<AtomicUsize>;

        // Define the structure of a ring buffer
        let len = 128;
        let async_ring = AsyncRing::new(len);
        let mut ring_builder = RingBufferBuilder::new();

        // The backing store we must allocate by ourselves
        let state: State = (0..len).map(|_| AtomicUsize::new(0)).collect();
        let state: Arc<State> = Arc::new(state);

        let (alloc_req, alloc) = ring_builder.define_client(&mut graph_builder);

        // Define a task that generates allocation requests
        graph_builder.define_task(TaskInfo {
            cell_uses: vec![alloc_req.use_as_producer()],
            task: task_from_closure((alloc_req,), |(alloc_req,), context| {
                *context.borrow_cell_mut(*alloc_req) = vec![AllocReq { size: 29, align: 1 }];

                Ok(())
            }),
        });

        // Define a task that writes to an object on a ring buffer
        let command = graph_builder.define_cell(None);
        graph_builder.define_task(TaskInfo {
            cell_uses: vec![alloc.use_as_consumer(), command.use_as_producer()],
            task: task_from_closure(
                (state.clone(), alloc, command),
                |(state, alloc, command), context| {
                    let alloc = &context.borrow_cell(*alloc)[0];

                    for i in alloc.clone() {
                        let cell = &state[i as usize];

                        // The object should be free at this point
                        assert_eq!(cell.load(Ordering::Relaxed), 0);

                        cell.store(1, Ordering::Relaxed);
                    }

                    // Simulate a command buffer generation
                    *context.borrow_cell_mut(*command) = Some(alloc.clone());

                    Ok(())
                },
            ),
        });

        // Define a task that simulates command buffer submission
        let gpu_result = graph_builder.define_cell(None);
        graph_builder.define_task(TaskInfo {
            cell_uses: vec![command.use_as_consumer(), gpu_result.use_as_producer()],
            task: task_from_closure(
                (state.clone(), command, gpu_result),
                |(state, command, gpu_result), context| {
                    let alloc = context.borrow_cell(*command).clone().unwrap();
                    let state = Arc::clone(state);

                    let (send, recv) = oneshot::channel();

                    // Spawn an asychonous task to simulate the behavior of GPU
                    let executor = xdispatch::Queue::global(xdispatch::QueuePriority::Default);
                    executor.after_ms(50, move || {
                        for i in alloc {
                            // Mark the object as unused
                            assert_eq!(state[i as usize].swap(0, Ordering::Relaxed), 1);
                        }

                        // Notify the completion
                        send.send(()).unwrap();
                    });

                    // Return a `Future`
                    let mut gpu_result = context.borrow_cell_mut(*gpu_result);
                    *gpu_result = Some(recv);

                    Ok(())
                },
            ),
        });

        ring_builder.add_to_graph_inner(&mut graph_builder, gpu_result, async_ring, |x| x);

        println!("{:#?}", graph_builder);

        let mut graph = graph_builder.build();
        println!("{:#?}", graph);

        let executor = xdispatch::Queue::global(xdispatch::QueuePriority::Default);

        // Execute the graph for multiple times. Despite the graph is executed
        // faster than the simulated GPU can handle, our mechanism makes sure
        // the CPU task won't overwrite an object when it's still in use by the
        // GPU.
        for _ in 0..12 {
            graph.run(&executor).unwrap();
        }
    }
}
