//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! This module provides `taskman`-based tasks for facilitating ring buffer
//! implementation.
use futures::{executor::block_on, Future};
use zangfx::{base as gfx, utils::CmdBufferResult};

use crate::taskman::{CellRef, GraphBuilder, GraphContext, Task, TaskInfo};

/// Facilitates ring buffer implementation.
///
/// This type is used to create tasks (using the `taskman` framework) for ring
/// buffer maintenance. They generate a buffer index (∈ `0..RingBuffer::len()`),
/// which can be used to index into a fixed-size ring buffer (of e.g. argument
/// tables) allocated by a client. They track the execution state of command
/// buffers to ensure no more than one agent (CPU or GPU) access the
/// ring buffer entry corresponding to the index.
#[derive(Debug)]
pub struct RingBuilder {
    len: usize,
    ring_index: CellRef<usize>,
}

impl RingBuilder {
    pub fn new<T>(graph_builder: &mut GraphBuilder<T>, len: usize) -> Self {
        assert!(len > 0);

        Self {
            len,
            ring_index: graph_builder.define_cell(0),
        }
    }

    /// Return the size of a ring buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return a cell reference that can be used to retrieve the ring buffer
    /// position (∈ `0..self.len()`) for the current run.
    pub fn ring_index(&self) -> CellRef<usize> {
        self.ring_index
    }

    /// Register tasks to `GraphBuilder`, consuming `self`.
    ///
    /// See [`add_to_graph_inner`] for details.
    pub fn add_to_graph(
        self,
        graph_builder: &mut GraphBuilder<gfx::Error>,
        cmd_buffer_result: CellRef<Option<CmdBufferResult>>,
    ) {
        self.add_to_graph_inner(graph_builder, cmd_buffer_result, |r| {
            r.expect("command buffer was unexpectedly cancelled")
        })
    }

    /// Register tasks to `GraphBuilder`, consuming `self`.
    ///
    /// This method registers two tasks:
    ///
    ///  - **The acquire task** locates the least recently used entry in the
    ///    ring buffer and updates `ring_index`. It waits for command buffer
    ///    completion if the entry is still in use.
    ///
    ///  - **The store task** receives a `Future` `R` via `cmd_buffer_result`
    ///    and associates it with the current ring buffer entry so the entry
    ///    won't be used again until `cmd_buffer_result` completes.
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
        transform_result: impl Fn(R::Output) -> Result<(), E> + Send + Sync + 'static,
    ) where
        R: Future + std::fmt::Debug + Send + Sync + 'static,
    {
        let state = graph_builder.define_cell(RingState {
            cb_results: (0..self.len).map(|_| None).collect(),
            index: 0,
        });
        let ring_index = self.ring_index;

        graph_builder.define_task(TaskInfo {
            task: Box::new(AcquireTask {
                state,
                output_ring_index: ring_index,
                transform_result,
            }),
            cell_uses: vec![state.use_as_producer(), ring_index.use_as_producer()],
        });

        graph_builder.define_task(TaskInfo {
            task: Box::new(StoreTask {
                state,
                input_cmd_buffer_result: cmd_buffer_result,
            }),
            cell_uses: vec![state.use_as_consumer(), cmd_buffer_result.use_as_consumer()],
        });
    }
}

#[derive(Debug)]
struct RingState<R> {
    cb_results: Vec<Option<R>>,
    index: usize,
}

struct AcquireTask<R, T> {
    state: CellRef<RingState<R>>,
    output_ring_index: CellRef<usize>,
    transform_result: T,
}

impl<R, T> std::fmt::Debug for AcquireTask<R, T>
where
    R: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("AcquireTask")
            .field("state", &self.state)
            .field("output_ring_index", &self.output_ring_index)
            .field("transform_result", &())
            .finish()
    }
}

impl<R, E, T> Task<E> for AcquireTask<R, T>
where
    R: Future + std::fmt::Debug + 'static,
    T: Fn(R::Output) -> Result<(), E> + Send + Sync,
{
    fn execute(&self, graph_context: &GraphContext) -> Result<(), E> {
        let mut state = graph_context.borrow_cell_mut(self.state);
        let index = state.index;

        if let Some(cb_result) = state.cb_results[index].take() {
            // Wait for the entry to be free
            (self.transform_result)(block_on(cb_result))?;
        }

        // Tell tasks which ring buffer entry to use for the current run
        *graph_context.borrow_cell_mut(self.output_ring_index) = index;

        Ok(())
    }
}

#[derive(Debug)]
struct StoreTask<R> {
    state: CellRef<RingState<R>>,
    input_cmd_buffer_result: CellRef<Option<R>>,
}

impl<R, E> Task<E> for StoreTask<R>
where
    R: std::fmt::Debug + 'static,
{
    fn execute(&self, graph_context: &GraphContext) -> Result<(), E> {
        let mut state = graph_context.borrow_cell_mut(self.state);
        let state = &mut *state;

        let cb_result = graph_context
            .borrow_cell_mut(self.input_cmd_buffer_result)
            .take()
            .expect("cmd_buffer_result is None");

        state.cb_results[state.index] = Some(cb_result);
        state.index += 1;
        if state.index == state.cb_results.len() {
            state.index = 0;
        }

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
        let ring_builder = RingBuilder::new(&mut graph_builder, 3);

        // The ring buffer we must allocate by ourselves
        let state: State = (0..ring_builder.len())
            .map(|_| AtomicUsize::new(0))
            .collect();
        let state: Arc<State> = Arc::new(state);

        // Define a task that writes to an object on a ring buffer
        let ring_index = ring_builder.ring_index();
        let command = graph_builder.define_cell(None);
        graph_builder.define_task(TaskInfo {
            cell_uses: vec![ring_index.use_as_consumer(), command.use_as_producer()],
            task: task_from_closure(
                (state.clone(), ring_index, command),
                |(state, ring_index, command), context| {
                    let i = *context.borrow_cell(*ring_index);

                    // The object should be free at this point
                    assert_eq!(state[i].load(Ordering::Relaxed), 0);

                    state[i].store(1, Ordering::Relaxed);

                    // Simulate a command buffer generation
                    *context.borrow_cell_mut(*command) = Some(i);

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
                    let i = context.borrow_cell(*command).unwrap();
                    let state = Arc::clone(state);

                    let (send, recv) = oneshot::channel();

                    // Spawn an asychonous task to simulate the behavior of GPU
                    let executor = xdispatch::Queue::global(xdispatch::QueuePriority::Default);
                    executor.after_ms(50, move || {
                        // Mark the object as unused
                        assert_eq!(state[i].swap(0, Ordering::Relaxed), 1);

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

        ring_builder.add_to_graph_inner(&mut graph_builder, gpu_result, |x| x);

        println!("{:#?}", graph_builder);

        let mut graph = graph_builder.build();
        println!("{:#?}", graph);

        let executor = xdispatch::Queue::global(xdispatch::QueuePriority::Default);

        // Execute the graph for multiple times. Despite the graph is executed
        // faster than the simulated GPU can handle, our mechanism makes sure
        // the CPU task won't overwrite an object when it's still in use by the
        // GPU.
        for _ in 0..6 {
            graph.run(&executor).unwrap();
        }
    }
}
