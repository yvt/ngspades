//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Defines [`Task`](crate::taskman::Task)s for comamnd buffer generation and
//! submission.
use arrayvec::ArrayVec;
use zangfx::{
    base as gfx,
    utils::{CmdBufferFutureExt, CmdBufferResult},
};

use crate::{
    passman::{ResourceId, Run, ScheduleBuilder, ScheduleRunner},
    taskman::{CellId, CellRef, CellUse, GraphBuilder, GraphContext, Task, TaskInfo},
};

/// Stores information required to generate CPU task objects for command buffer
/// generation and submission.
///
/// `CmdBufferTaskBuilder` generates two tasks: command buffer encoding and
/// command buffer submission. The clients register consuming `CellUse`s to
/// each task to ensure corresponding data is generated before their respective
/// processing stage.
///
/// It internally manages a `passman::ScheduleBuilder` to construct a GPU task
/// graph (which is then used by the command buffer encoding task to generate
/// commands). The clients access it via the `schedule_builder` method to
/// register GPU pass objects.
///
/// During the command buffer generation, the pass objects can access
/// cells through `PassContext` (which is a type alias of
/// `taskman::GraphContext`). The clients must use the `add_encoding_dependency`
/// method to register such cells for consumption by the command buffer encoding
/// task.
///
/// Finally, those two tasks are registered to a supplied `GraphBuilder` when
/// the `add_to_graph` method is called. This also allocates device memory
/// required to run GPU operations.
///
/// TODO: Coordinate with other subsystems that require memory allocation.
#[derive(Debug)]
pub struct CmdBufferTaskBuilder {
    schedule_builder: ScheduleBuilder<PassContext>,
    encode_cell_uses: Vec<CellUse>,
    submit_cell_uses: Vec<CellUse>,
}

/// The type of context data passed to `Pass`es (GPU pass objects).
///
/// This represents a reference to a `GraphContext`. Pass implementations can
/// use this to access cell contents during command buffer encoding.
pub type PassContext = GraphContext;

#[derive(Debug, Clone)]
pub struct CmdBufferTaskCellSet {
    /// If a fence is stored to this cell before a graph is run, the fence
    /// will be updated after the command buffer execution.
    pub update_fence: CellRef<Option<gfx::FenceRef>>,

    /// If a closure is stored to this cell, the closure will be called to bind
    /// late-bound resources.
    pub late_resource_binder: CellRef<LateResourceBinderCell>,

    /// `CmdBufferResult` will be stored after command buffer submission. The
    /// number of elements is determined by the `num_result_cells` parameter
    /// of `CmdBufferTaskBuffer::add_to_graph`.
    pub cmd_buffer_results: Vec<CellRef<Option<CmdBufferResult>>>,
}

impl CmdBufferTaskBuilder {
    pub fn new() -> Self {
        Self {
            schedule_builder: ScheduleBuilder::new(),
            encode_cell_uses: Vec::new(),
            submit_cell_uses: Vec::new(),
        }
    }

    /// Get a `ScheduleBuilder` used to consturct a GPU task graph.
    pub fn schedule_builder(&mut self) -> &mut ScheduleBuilder<PassContext> {
        &mut self.schedule_builder
    }

    /// Register a cell to be implicitly consumed by a command buffer
    /// generation task.
    pub fn add_encoding_dependency(&mut self, cell_id: &CellId) {
        self.encode_cell_uses.push(cell_id.use_as_consumer());
    }

    /// Register a cell to be implicitly consumed by a command buffer
    /// submission task. This is intended to be used wtih host writes into
    /// host-visible memory.
    pub fn add_submission_dependency(&mut self, cell_id: &CellId) {
        self.submit_cell_uses.push(cell_id.use_as_consumer());
    }

    /// Register tasks to `GraphBuilder`, consuming `self`.
    pub fn add_to_graph(
        mut self,
        device: &gfx::DeviceRef,
        queue: &gfx::CmdQueueRef,
        graph_builder: &mut GraphBuilder<gfx::Error>,
        output_resources: &[&ResourceId],
        num_result_cells: usize,
    ) -> gfx::Result<CmdBufferTaskCellSet> {
        // Finalize the GPU task graph
        let schedule = self.schedule_builder.schedule(output_resources);
        let schedule_runner = schedule.instantiate(device, queue)?;
        assert_eq!(schedule_runner.num_output_fences(), 1);

        let schedule_runner = graph_builder.define_cell(schedule_runner);

        // The cell used to store an encoded command buffer
        let cmd_buffer_cell = graph_builder.define_cell(None);

        let prev_fence_cell = graph_builder.define_cell(None);
        let update_fence = graph_builder.define_cell(None);

        let late_resource_binder = graph_builder.define_cell(LateResourceBinderCell::default());

        let cmd_buffer_results: Vec<_> = (0..num_result_cells)
            .map(|_| graph_builder.define_cell(None))
            .collect();

        // Command buffer generation
        self.encode_cell_uses
            .push(cmd_buffer_cell.use_as_producer());
        self.encode_cell_uses
            .push(schedule_runner.use_as_producer());
        graph_builder.define_task(TaskInfo {
            cell_uses: self.encode_cell_uses,
            task: Box::new(CbEncodeTask {
                cmd_buffer_cell,
                prev_fence_cell,
                update_fence,
                late_resource_binder,
                queue: queue.clone(),
                schedule_runner,
            }),
        });

        // Command buffer submission
        self.submit_cell_uses
            .push(cmd_buffer_cell.use_as_consumer());
        graph_builder.define_task(TaskInfo {
            cell_uses: self.submit_cell_uses,
            task: Box::new(CbSubmitTask {
                cmd_buffer_cell,
                cmd_buffer_results: cmd_buffer_results.clone(),
            }),
        });

        Ok(CmdBufferTaskCellSet {
            update_fence,
            cmd_buffer_results,
            late_resource_binder,
        })
    }
}

#[derive(Debug)]
pub struct CbEncodeTask {
    cmd_buffer_cell: CellRef<Option<gfx::CmdBufferRef>>,
    prev_fence_cell: CellRef<Option<gfx::FenceRef>>,
    update_fence: CellRef<Option<gfx::FenceRef>>,
    late_resource_binder: CellRef<LateResourceBinderCell>,
    queue: gfx::CmdQueueRef,
    schedule_runner: CellRef<ScheduleRunner<PassContext>>,
}

#[derive(Debug)]
struct CbSubmitTask {
    cmd_buffer_cell: CellRef<Option<gfx::CmdBufferRef>>,
    cmd_buffer_results: Vec<CellRef<Option<CmdBufferResult>>>,
}

impl Task<gfx::Error> for CbEncodeTask {
    fn execute(&self, graph_context: &GraphContext) -> gfx::Result<()> {
        let mut cmd_buffer = self.queue.new_cmd_buffer()?;

        let mut schedule_runner = graph_context.borrow_cell_mut(self.schedule_runner);
        let mut prev_fence_cell = graph_context.borrow_cell_mut(self.prev_fence_cell);
        let mut update_fence = graph_context.borrow_cell_mut(self.update_fence);

        // Prepare the run
        let mut run = schedule_runner.run()?;

        // Override the output fence
        let output_fence;
        {
            use ngsgamegfx_common::iterator_mut::IteratorMut;

            let mut iter = run.output_fences_mut();
            let output_fence_place: &mut gfx::FenceRef = iter.next().unwrap();
            if let Some(fence) = update_fence.take() {
                output_fence = fence.clone();
                *output_fence_place = fence;
            } else {
                output_fence = output_fence_place.clone();
            }
        }

        // Bind late-bound resouce
        let mut late_resource_binder = graph_context.borrow_cell_mut(self.late_resource_binder);
        if let Some(binder) = late_resource_binder.cell.take() {
            binder(&mut run);
        }

        // Encode commands
        let input_fence = prev_fence_cell.take();
        let input_fences: ArrayVec<[_; 1]> = input_fence.iter().collect();

        run.encode(&mut cmd_buffer, &input_fences, graph_context)?;

        // Store the fence
        *prev_fence_cell = Some(output_fence);

        // Store the command buffer
        *graph_context.borrow_cell_mut(self.cmd_buffer_cell) = Some(cmd_buffer);

        Ok(())
    }
}

impl Task<gfx::Error> for CbSubmitTask {
    fn execute(&self, graph_context: &GraphContext) -> gfx::Result<()> {
        let mut cmd_buffer: gfx::CmdBufferRef = graph_context
            .borrow_cell_mut(self.cmd_buffer_cell)
            .take()
            .expect("cb is missing");

        // Create a `Future` representing the result of command buffer execution
        for &cmd_buffer_result_cell in self.cmd_buffer_results.iter() {
            *graph_context.borrow_cell_mut(cmd_buffer_result_cell) = Some(cmd_buffer.result());
        }

        cmd_buffer.commit()?;

        Ok(())
    }
}

/// Used to pass a closure for binding late-bound resources to a command buffer
/// generation task.
///
/// `FnOnce` is not `Debug`, thus it does not implement [`Cell`]. This type
/// wraps `FnOnce` so it can be passed via a cell.
///
/// [`Cell`]: crate::taskman::Cell
#[derive(Default)]
pub struct LateResourceBinderCell {
    cell: Option<Box<dyn FnOnce(&mut Run<PassContext>) + Send + Sync>>,
}

impl std::fmt::Debug for LateResourceBinderCell {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("LateResourceBinderCell")
            .field("cell", &self.cell.as_ref().map(|_| ()))
            .finish()
    }
}

impl LateResourceBinderCell {
    pub fn set(&mut self, x: impl FnOnce(&mut Run<PassContext>) + Send + Sync + 'static) {
        self.cell = Some(Box::new(x));
    }
}
