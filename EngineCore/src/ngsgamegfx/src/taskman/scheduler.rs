//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use atomic_refcell::AtomicRefCell;
use owning_ref::{OwningRef, OwningRefMut};
use parking_lot::Mutex;
use std::{
    any::Any,
    panic,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread,
};

use super::{Cell, CellId, CellRef, Task, TaskInfo};

use crate::utils::owning_ref::AssertStableAddress;

#[cfg(test)]
#[path = "./scheduler_test.rs"]
mod scheduler_test;

/// Stores the description of a task graph and serves as a builder object of
/// [`Graph`].
#[derive(Debug)]
pub struct GraphBuilder {
    cells: Vec<BuilderCell>,
    tasks: Vec<BuilderTask>,
}

#[derive(Debug)]
struct BuilderTask {
    info: TaskInfo,
}

impl From<TaskInfo> for BuilderTask {
    fn from(x: TaskInfo) -> Self {
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

impl GraphBuilder {
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

    pub fn define_task(&mut self, task: TaskInfo) {
        self.tasks.push(task.into());
    }

    /// Construct a `Graph`, consuming `self`.
    pub fn build(mut self) -> Graph {
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

        let inner = GraphInner {
            tasks,
            cells: cells
                .into_iter()
                .map(|cell| AtomicRefCell::new(cell.initializer))
                .collect(),
            poisoned: AtomicBool::new(false),
        };

        Graph {
            inner: Arc::new(inner),
        }
    }
}

#[derive(Debug)]
pub struct Graph {
    inner: Arc<GraphInner>,
}

#[derive(Debug)]
struct GraphInner {
    tasks: Vec<GraphTask>,
    cells: Vec<AtomicRefCell<Box<dyn Cell>>>,

    /// A flag indicating if a panic has ever occured while running this graph.
    poisoned: AtomicBool,
}

#[derive(Debug)]
struct GraphTask {
    task: Box<dyn Task>,

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

#[derive(Debug)]
struct GraphRun {
    num_pending_tasks: AtomicUsize,
    initiating_thread: thread::Thread,

    /// The cell to store the error information if a panic has occured during
    /// the current run.
    error: Mutex<Option<Box<dyn Any + Send + 'static>>>,
}

pub trait Executor {
    fn spawn(&self, f: impl FnOnce(&Self) + Send + 'static);
}

#[derive(Debug)]
pub struct GraphContext<'a> {
    cells: &'a [AtomicRefCell<Box<dyn Cell>>],
}

impl Graph {
    /// # Panics
    ///
    ///  - Might panic if `Graph` is in the "poisoned" state.
    ///
    pub fn borrow_cell_mut(&mut self, cell_id: CellId) -> &mut dyn Cell {
        let inner: &mut GraphInner = Arc::get_mut(&mut self.inner).unwrap();

        inner.cells[cell_id.0].get_mut()
    }

    /// Run a task graph. Block the current thread until all tasks complete
    /// execution.
    ///
    /// This method blocks forever if there is a cyclic dependency in the
    /// task graph.
    ///
    /// # Panics
    ///
    ///  - If any of tasks panics, it will be reported back to the initiating
    ///    thread. In this case, `run` does not wait for the completion of
    ///    outstanding tasks. Furthermore, `Graph` will be transitioned into
    ///    the "poisoned" state.
    ///  - Panics if `Graph` is in the "poisoned" state.
    ///
    pub fn run(&mut self, executor: &impl Executor) {
        if self.inner.poisoned.load(Ordering::Relaxed) {
            panic!("poisoned");
        }

        // Prepare the graph run
        let run: Arc<GraphRun>;
        {
            let inner: &mut GraphInner = Arc::get_mut(&mut self.inner).unwrap();

            for task in inner.tasks.iter() {
                task.num_blocking_tasks
                    .store(task.max_num_blocking_tasks, Ordering::Relaxed);
            }

            run = Arc::new(GraphRun {
                num_pending_tasks: AtomicUsize::new(inner.tasks.len()),
                initiating_thread: thread::current(),
                error: Mutex::new(None),
            });
        }

        // Spawn initial tasks
        for (i, task) in self.inner.tasks.iter().enumerate() {
            if task.max_num_blocking_tasks > 0 {
                break;
            }
            Self::spawn_task(executor, &self.inner, &run, i);
        }

        loop {
            thread::park();

            // Figure out why the current thread was unparked.
            if self.inner.poisoned.load(Ordering::Relaxed) {
                // One of the tasks has panicked. Propagate the panic.
                panic::resume_unwind(run.error.lock().take().unwrap());
            }

            if run.num_pending_tasks.load(Ordering::Relaxed) == 0 {
                // No more executable tasks
                break;
            }
        }

        assert!(Arc::get_mut(&mut self.inner).is_some());
    }

    fn spawn_task(
        executor: &impl Executor,
        inner: &Arc<GraphInner>,
        run: &Arc<GraphRun>,
        task_id: usize,
    ) {
        let inner = panic::AssertUnwindSafe(Arc::clone(inner));
        let run = Arc::clone(run);

        executor.spawn(move |executor| {
            // Fail-fast
            if inner.poisoned.load(Ordering::Relaxed) {
                return;
            }

            let result = panic::catch_unwind(|| {
                let graph_context = GraphContext {
                    cells: &inner.cells[..],
                };
                inner.tasks[task_id].task.execute(&graph_context);
            });

            if let Err(err) = result {
                // The task has panicked - report back the error
                if !inner.poisoned.swap(true, Ordering::Relaxed) {
                    *run.error.lock() = Some(err);
                    run.initiating_thread.unpark();
                }
                return;
            }

            // Unblock dependent tasks
            for &i in inner.tasks[task_id].unblocks_tasks.iter() {
                if inner.tasks[i]
                    .num_blocking_tasks
                    .fetch_sub(1, Ordering::Relaxed)
                    == 1
                {
                    Self::spawn_task(executor, &inner, &run, i);
                }
            }

            // Drop `inner` before touching `num_pending_tasks` so that
            // the initating thread has an unique reference to the `GraphInner`
            // after being unparked.
            drop(inner);

            // Is the run complete?
            if run.num_pending_tasks.fetch_sub(1, Ordering::Relaxed) == 1 {
                run.initiating_thread.unpark();
            }
        });
    }
}

impl GraphContext<'_> {
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
        let cell_ref = self.cells[cell_ref.id().0].borrow_mut();
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
        let cell_ref = self.cells[cell_ref.id().0].borrow();
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
        let cell_ref = self.cells[cell_id.0].borrow_mut();
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
        let cell_ref = self.cells[cell_id.0].borrow();
        OwningRef::new(AssertStableAddress(cell_ref)).map(|x| &**x)
    }
}
