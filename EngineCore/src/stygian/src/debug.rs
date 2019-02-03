//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/// A trait for observing the internal behaviour of Stygian.
pub trait Trace: Clone {
    // TODO
}

/// `Trace` implementation that does nothing.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NoTrace;

impl Trace for NoTrace {}
