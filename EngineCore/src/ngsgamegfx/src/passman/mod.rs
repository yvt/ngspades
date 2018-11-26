//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! The render pass manager.
//!
//! # Terminology
//!
//! TODO
//!
//!
//! # Resource allocation
//!
//! ## Memory resources
//!
//! TODO
//!
//!
//! ## Argument tables
//!
//! TODO
//!
//!
//! ## Late-bound resources
//!
//! Resources on `ScheduleBuilder` can be marked as **late-bound resources**.
//! The scheduler does not instantiate resources marked as late-bound, and
//! `PassInstantiationContext::get_resource` will return `None` for such
//! resources. The factory method of a pass **may or may not** support
//! late-bound resources.
//!
//! Late-bound resources serve as a placeholder of resources which are to be
//! provided for every frame (rather than being constructed along with `Pass`es).
//! The caller of `ScheduleRunner::run` must "fill the places" by
//! calling `bind` of the returned `Run` so that they are finally available to
//! `Pass::encoder` via `PassEncodingContext::get_resource`.
//!
//! The intended use case of this feature is the final output image, for which
//! we must write on a specific image provided by an external subsystem.
//!
//! This feature is not intended to be used for argument table resources.
//!
mod info;
mod resources;
mod scheduler;
pub use self::info::*;
pub use self::resources::*;
pub use self::scheduler::*;
