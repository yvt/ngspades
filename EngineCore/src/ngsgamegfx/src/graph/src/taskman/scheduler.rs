//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use atomic_refcell::AtomicRefCell;
use cryo::{with_cryo, CryoRef};
use opaque_typedef_macros::OpaqueTypedefUnsized;
use owning_ref::{OwningRef, OwningRefMut};
use parking_lot::Mutex;
use std::{
    any::Any,
    panic,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use super::{Cell, CellId, CellRef, Task, TaskInfo};

use ngsgamegfx_common::owning_ref::AssertStableAddress;

#[cfg(test)]
#[path = "./scheduler_test.rs"]
mod scheduler_test;

/// Stores the description of a task graph and serves as a builder object of
/// [`Graph`].
#[derive(Debug)]
pub struct GraphBuilder<E> {
    cells: Vec<BuilderCell>,
    tasks: Vec<BuilderTask<E>>,
}

#[derive(Debug)]
struct BuilderTask<E> {
    info: TaskInfo<E>,
}

impl<E> From<TaskInfo<E>> for BuilderTask<E> {
    fn from(x: TaskInfo<E>) -> Self {
        Self { info: x }
    }
}

#[derive(Debug)]
struct BuilderCell {
    initializer: Box<dyn Cell>,
    consuming_tasks: Vec<usize>,
}

impl From<Box<dyn Cell>> for BuilderCell {
    fn from(x: Box<dyn Cell>) -> Self {
        Self {
            initializer: x,
            consuming_tasks: Vec::new(),
        }
    }
}

impl<E> GraphBuilder<E> {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            tasks: Vec::new(),
        }
    }

    /// Define a cell.
    ///
    /// Returns the `CellId` representing the newly defined
    /// resource. The returned `CellId` only pertains to `self`.
    pub fn define_cell<T: Cell>(&mut self, initializer: T) -> CellRef<T> {
        let next_index = self.cells.len();
        self.cells
            .push((Box::new(initializer) as Box<dyn Cell>).into());
        CellRef::new(CellId(next_index))
    }

    pub fn define_task(&mut self, task: TaskInfo<E>) {
        self.tasks.push(task.into());
    }

    /// Construct a `Graph`, consuming `self`.
    pub fn build(mut self) -> Graph<E> {
        for (i, task) in self.tasks.iter().enumerate() {
            for cell_use in &task.info.cell_uses {
                if !cell_use.produce {
                    self.cells[cell_use.cell.0].consuming_tasks.push(i);
                }
            }
        }

        let cells = self.cells;

        let mut tasks: Vec<_> = self
            .tasks
            .into_iter()
            .map(|task| {
                let mut unblocks_tasks = Vec::new();

                for cell_use in &task.info.cell_uses {
                    if cell_use.produce {
                        unblocks_tasks
                            .extend(cells[cell_use.cell.0].consuming_tasks.iter().cloned());
                    }
                }

                unblocks_tasks.sort();
                unblocks_tasks.dedup();

                GraphTask {
                    task: task.info.task,
                    max_num_blocking_tasks: 0,
                    unblocks_tasks,
                    num_blocking_tasks: AtomicUsize::new(0),
                }
            })
            .collect();

        // Fill `max_num_blocking_tasks`
        for task in &tasks {
            for &i in &task.unblocks_tasks {
                tasks[i].num_blocking_tasks.fetch_add(1, Ordering::Relaxed);
            }
        }
        for task in &mut tasks {
            task.max_num_blocking_tasks = task.num_blocking_tasks.load(Ordering::Relaxed);
        }

        Graph {
            tasks,
            cells: cells
                .into_iter()
                .map(|cell| AtomicRefCell::new(cell.initializer))
                .collect(),
            poisoned: AtomicBool::new(false),
            panic: Mutex::new(None),
            error: Mutex::new(None),
        }
    }
}

#[derive(Debug)]
pub struct Graph<E> {
    tasks: Vec<GraphTask<E>>,
    cells: Vec<AtomicRefCell<Box<dyn Cell>>>,

    /// A flag indicating if a panic has ever occured while running this graph.
    poisoned: AtomicBool,

    /// The cell to store the panic information if a panic has occured during
    /// the current run.
    panic: Mutex<Option<Box<dyn Any + Send + 'static>>>,

    /// The cell to store the error information if an error has occured during
    /// the current run.
    error: Mutex<Option<E>>,
}

#[derive(Debug)]
struct GraphTask<E> {
    task: Box<dyn Task<E>>,

    /// The number of tasks in the graph that must be completed before this task
    /// can start.
    max_num_blocking_tasks: usize,

    /// The list of tasks dependent on the output of this task.
    unblocks_tasks: Vec<usize>,

    // The field below is relevant to a particular run
    /// The number of tasks that must be completed before this task can start
    /// in the current run. Starts at `max_num_blocking_tasks`.
    num_blocking_tasks: AtomicUsize,
}

pub trait Executor {
    fn spawn(&self, f: impl FnOnce(&Self) + Send + 'static);
}

// We use `OpaqueTypedefUnsized` to make `GraphContext` `'static` so it can be
// passed as context data of `passman::Pass`.
#[derive(Debug, OpaqueTypedefUnsized)]
#[repr(transparent)]
#[opaque_typedef(allow_mut_ref)]
#[opaque_typedef(derive(AsMutDeref, AsMutSelf, AsRefDeref, AsRefSelf, IntoInner, FromInner))]
pub struct GraphContext([AtomicRefCell<Box<dyn Cell>>]);

impl<E: Send + 'static> Graph<E> {
    /// Mutably borrow a cell using a strongly-typed cell identifier.
    ///
    /// # Panics
    ///
    ///  - This method panics if the concrete type of `cell_ref` does not match
    ///    that of the cell specified by `cell_ref`.
    ///  - Might panic if the `Graph` is in the "poisoned" state.
    ///
    pub fn borrow_cell_mut<T: Cell>(&mut self, cell_id: CellRef<T>) -> &mut T {
        (self.cells[cell_id.0].get_mut())
            .downcast_mut::<T>()
            .expect("type mismatch")
    }

    /// Run a task graph. Block the current thread until all tasks complete
    /// execution.
    ///
    /// This method blocks forever if there is a cyclic dependency in the
    /// task graph.
    ///
    /// Returns `Err(e)` if any of tasks return `Err(e)`. When this happens,
    /// `Graph` will be transitioned into the "poisoned" state.
    /// If more than one tasks err, only one of the error values will be
    /// returned and the rest is discarded.
    ///
    /// # Panics
    ///
    ///  - If any of tasks panic, it will be reported back to the initiating
    ///    thread. `Graph` will be transitioned into the "poisoned" state.
    ///  - Panics if `Graph` is in the "poisoned" state.
    ///
    pub fn run(&mut self, executor: &impl Executor) -> Result<(), E> {
        if self.poisoned.load(Ordering::Relaxed) {
            panic!("poisoned");
        }

        // Prepare the graph run
        for task in self.tasks.iter() {
            task.num_blocking_tasks
                .store(task.max_num_blocking_tasks, Ordering::Relaxed);
        }

        // Use `cryo` to capture local variables in
        // a `'static` closure (the one passed to `Executor` in `spawn_task`).
        with_cryo(self, |cryo_this| {
            let this_ref = cryo_this.borrow();

            // Spawn initial tasks
            for (i, task) in self.tasks.iter().enumerate() {
                if task.max_num_blocking_tasks > 0 {
                    break;
                }
                Self::spawn_task(executor, &this_ref, i);
            }
        });

        // `with_cryo` will not return until all uses of `CryoRef` are done.

        if self.poisoned.load(Ordering::Relaxed) {
            // Panics take precedence over errors
            if let Some(x) = self.panic.get_mut().take() {
                // One of the tasks has panicked. Propagate the panic.
                panic::resume_unwind(x);
            } else {
                // One of the tasks has raised an error. Propagate the error.
                return Err(self.error.get_mut().take().unwrap());
            }
        }

        Ok(())
    }

    fn spawn_task(executor: &impl Executor, this: &CryoRef<Self>, task_id: usize) {
        let mut this = panic::AssertUnwindSafe(CryoRef::clone(this));

        executor.spawn(move |executor| {
            // Fail-fast
            if this.poisoned.load(Ordering::Relaxed) {
                return;
            }

            let result = panic::catch_unwind(|| {
                let graph_context = <&GraphContext>::from(&this.cells[..]);
                this.tasks[task_id].task.execute(graph_context)
            });

            match result {
                Err(panic) => {
                    // The task has panicked - report back the error
                    *this.panic.lock() = Some(panic);
                    this.poisoned.store(true, Ordering::Relaxed);
                    return;
                }
                Ok(Err(error)) => {
                    if !this.poisoned.swap(true, Ordering::Relaxed) {
                        *this.error.lock() = Some(error);
                    }
                    return;
                }
                Ok(Ok(())) => {}
            }

            // Unblock dependent tasks
            for &i in this.tasks[task_id].unblocks_tasks.iter() {
                if this.tasks[i]
                    .num_blocking_tasks
                    .fetch_sub(1, Ordering::Relaxed)
                    == 1
                {
                    Self::spawn_task(executor, &this, i);
                }
            }
        });
    }
}

impl GraphContext {
    /// Mutably borrow a cell using a strongly-typed cell identifier.
    ///
    /// The calling task must have a producing use of the cell defined when
    /// registered to the task graph.
    /// Otherwise, calling this method might interfere with the operation of
    /// the task runner.
    ///
    /// # Panics
    ///
    /// This method panics if the concrete type of `cell_ref` does not match
    /// that of the cell specified by `cell_ref`.
    pub fn borrow_cell_mut<'a, T: Any>(
        &'a self,
        cell_ref: CellRef<T>,
    ) -> impl std::ops::Deref<Target = T> + std::ops::DerefMut + 'a {
        let cell_ref = self.0[cell_ref.id().0].borrow_mut();
        OwningRefMut::new(AssertStableAddress(cell_ref))
            .map_mut(|x| x.downcast_mut::<T>().expect("type mismatch"))
    }

    /// Borrow a cell using a strongly-typed cell identifier.
    ///
    /// The calling task must have a use of the cell defined when
    /// registered to the task graph.
    /// Otherwise, calling this method might interfere with the operation of
    /// the task runner.
    ///
    /// # Panics
    ///
    /// This method panics if the concrete type of `cell_ref` does not match
    /// that of the cell specified by `cell_ref`.
    pub fn borrow_cell<'a, T: Any>(
        &'a self,
        cell_ref: CellRef<T>,
    ) -> impl std::ops::Deref<Target = T> + 'a {
        let cell_ref = self.0[cell_ref.id().0].borrow();
        OwningRef::new(AssertStableAddress(cell_ref))
            .map(|x| x.downcast_ref::<T>().expect("type mismatch"))
    }

    /// Mutably borrow a cell using an untyped cell identifier.
    ///
    /// The calling task must have a producing use of the cell defined when
    /// registered to the task graph.
    /// Otherwise, calling this method might interfere with the operation of
    /// the task runner.
    pub fn borrow_dyn_cell_mut<'a>(
        &'a self,
        cell_id: CellId,
    ) -> impl std::ops::Deref<Target = dyn Cell> + std::ops::DerefMut + 'a {
        let cell_ref = self.0[cell_id.0].borrow_mut();
        OwningRefMut::new(AssertStableAddress(cell_ref)).map_mut(|x| &mut **x)
    }

    /// Borrow a cell using an untyped cell identifier.
    ///
    /// The calling task must have a use of the cell defined when
    /// registered to the task graph.
    /// Otherwise, calling this method might interfere with the operation of
    /// the task runner.
    pub fn borrow_dyn_cell<'a>(
        &'a self,
        cell_id: CellId,
    ) -> impl std::ops::Deref<Target = dyn Cell> + 'a {
        let cell_ref = self.0[cell_id.0].borrow();
        OwningRef::new(AssertStableAddress(cell_ref)).map(|x| &**x)
    }
}
