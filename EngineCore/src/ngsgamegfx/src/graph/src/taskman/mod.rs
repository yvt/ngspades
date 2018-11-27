//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! The CPU task graph manager.
//!
//! # Terminology
//!
//! - A **graph** is defined by a set of tasks that should be executed when
//!   the graph is run. Tasks in a graph may share a block of data via cells.
//! - **Tasks** are a unit of execution.
//! - **Cells** are used for exchanging data between tasks as well as for
//!   enforcing the execution order of tasks. The contents of cells may persist
//!   between multiple runs of a graph though this is not guaranteed.
//!
//! # Relationship to the GPU pass manager
//!
//! The only fundamental difference in their goals is that this one produces a
//! task sequence for CPU execution while the other one produces for GPU.
//! However, because of how a GPU works differently from a CPU, the same goes
//! for the task scheduler.
//!
//! In most common use cases, the CPU work required for GPU command generation
//! (coordinated by the GPU pass manager) is expressed as a single (or more)
//! task in a CPU task graph. The command buffer submission is better expressed
//! as a separate task because in this way per-frame data generation and
//! GPU command generation can be executed simultaneously using multiple CPU
//! cores.
#[macro_use]
mod info;
mod scheduler;
mod xdispatch;
pub use self::info::*;
pub use self::scheduler::*;
