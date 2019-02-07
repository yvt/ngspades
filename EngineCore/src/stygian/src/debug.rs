//
// Copyright 2019 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::{Vector2, Vector3};

/// A trait for observing the internal behaviour of Stygian.
///
/// `Trace` is intended to be cloned for each thread when executing multiple
/// threads. The methods take a mutable reference as the receiver so that the
/// implementation can have a per-thread log buffer that can be written without
/// locking.
///
/// The demo program uses this trait to visualize the inner workings.
pub trait Trace: Clone {
    /// Return whether calls to `terrainrast_sample` can be skipped.
    fn wants_terrainrast_sample(&mut self) -> bool {
        false
    }

    /// Called for every generated terrainrast sample.
    fn terrainrast_sample(&mut self, _vertices: &[Vector3<f32>; 4]) {}

    /// Return whether calls to `opticast_sample` can be skipped.
    fn wants_opticast_sample(&mut self) -> bool {
        false
    }

    /// Called for every rasterized terrainrast sample.
    fn opticast_sample(&mut self, _vertices: &[Vector3<f32>; 4], _depth: f32) {}

    fn opticast_span(&mut self, _pos: Vector2<u32>, _size: u32, _z: std::ops::Range<u32>) {}
}

/// `Trace` implementation that does nothing.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NoTrace;

impl Trace for NoTrace {}
