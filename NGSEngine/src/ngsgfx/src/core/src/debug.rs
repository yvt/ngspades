//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use cgmath::Vector3;

pub trait Marker {
    fn set_label(&self, label: Option<&str>) {}
}

/// Represents a debug marker or group that can be encoded into a command buffer.
#[derive(Debug, Copy, Clone)]
pub struct DebugMarker<'a> {
    name: &'a str,
    color: Option<Vector3<f32>>,
}

impl<'a> DebugMarker<'a> {
    #[inline]
    pub fn new(name: &'a str) -> Self {
        Self { name, color: None }
    }

    #[inline]
    pub fn with_color(&self, red: f32, green: f32, blue: f32) -> Self {
        Self {
            color: Some(Vector3::new(red, green, blue)),
            ..*self
        }
    }

    #[inline]
    pub fn name(&self) -> &'a str {
        self.name
    }

    #[inline]
    pub fn color(&self) -> &Option<Vector3<f32>> {
        &self.color
    }
}
