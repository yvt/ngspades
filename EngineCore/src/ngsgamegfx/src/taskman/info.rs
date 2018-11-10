//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::any::Any;

use super::Context;

/// Represents a task.
#[derive(Debug)]
pub struct TaskInfo {
    pub cell_uses: Vec<CellUse>,
    pub task: Box<dyn Task>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellUse {
    pub cell: CellId,

    /// Indicates whether this use indicates the production of the cell
    /// contents.
    ///
    /// For each cell, only one producing use can be defined in a graph.
    /// The producing task is responsible for producing the contents of the
    /// cell.
    pub produce: bool,
}

impl CellId {
    pub fn use_as_producer(self) -> CellUse {
        CellUse {
            cell: self,
            produce: true,
        }
    }

    pub fn use_as_consumer(self) -> CellUse {
        CellUse {
            cell: self,
            produce: false,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CellId(pub(super) usize);

/// The contents of a cell. This is automatically implemented on all compatible
/// types.
pub trait Cell: std::any::Any + Send + Sync + std::fmt::Debug {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync);
    fn as_any_mut(&mut self) -> &mut (dyn std::any::Any + Send + Sync);
}

impl<T: std::any::Any + Send + Sync + std::fmt::Debug> Cell for T {
    fn as_any(&self) -> &(dyn std::any::Any + Send + Sync) {
        self
    }
    fn as_any_mut(&mut self) -> &mut (dyn std::any::Any + Send + Sync) {
        self
    }
}

impl dyn Cell {
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        (*self).as_any().downcast_ref()
    }

    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        (*self).as_any_mut().downcast_mut()
    }
}

pub trait Task: std::fmt::Debug + Send + Sync {
    /// Execute the task.
    fn execute(&self, context: &Context);
}
