//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{any::Any, marker::PhantomData};

use super::GraphContext;

use crate::utils::any::AsAnySendSync;

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

/// A strongly typed version of `CellId`.
///
/// This is a simple wrapper around `CellId` that adds concrete type
/// information.
///
/// The reason that this is defined as a type alias is to circumvent the
/// restrictions of `derive` macros that trait bounds are not generated properly
/// for tricky cases of generic types.
pub type CellRef<T> = CellRefInner<fn(T) -> T>;

/// The internal implementation of `CellRef`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellRefInner<T> {
    id: CellId,
    _phantom: PhantomData<T>,
}

impl<T> CellRef<T> {
    pub fn new(id: CellId) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }

    /// Get a raw (untyped) cell identifier.
    pub fn id(&self) -> CellId {
        self.id
    }
}

impl<T> std::ops::Deref for CellRef<T> {
    type Target = CellId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

/// Points a `Cell` in a particular instance of task graph.
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct CellId(pub(super) usize);

/// The contents of a cell. This is automatically implemented on all compatible
/// types.
pub trait Cell: AsAnySendSync + std::fmt::Debug {}

impl<T: AsAnySendSync + std::fmt::Debug> Cell for T {}

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
    fn execute(&self, graph_context: &GraphContext);
}
