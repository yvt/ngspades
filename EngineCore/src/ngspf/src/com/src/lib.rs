//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsCOM bridges to [NgsPF](../ngspf/index.html).
extern crate atomic_refcell;
extern crate cgmath;
extern crate rgb;
#[macro_use]
extern crate lazy_static;
extern crate send_cell;

extern crate ngsbase;
extern crate cggeom;
#[macro_use]
extern crate ngscom;
extern crate ngspf_core as core;
extern crate ngspf_viewport as viewport;

mod context;
mod nodes;
mod workspace;

pub use ngsbase::{ILayer, INodeGroup, IPresentationContext, IWindow, IWindowListener, IWorkspace};

pub use context::ComContext;
pub use nodes::{INodeRef, INodeRefTrait};
pub use workspace::ComWorkspace;

/// `HResult` values generated by NgsPF.
///
/// ## References
///
/// - [Structure of COM Error Codes]
///
/// [Structure of COM Error Codes]: https://msdn.microsoft.com/en-us/library/windows/desktop/ms690088(v=vs.85).aspx
///
pub mod hresults {
    use ngscom::HResult;

    /// Unable to modify a property because the node is already materialized.
    pub const E_PF_NODE_MATERIALIZED: HResult = HResult(0x80410001u32 as i32);

    /// Specified object is not a node, or originates from a different context.
    pub const E_PF_NOT_NODE: HResult = HResult(0x80410002u32 as i32);

    /// The object is currently in use by another thread.
    pub const E_PF_LOCKED: HResult = HResult(0x80410003u32 as i32);

    /// The object is not owned by the current thread.
    pub const E_PF_THREAD: HResult = HResult(0x80410004u32 as i32);
}
