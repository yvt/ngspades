//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a NgsPF port type for embedding a NgsGameGFX viewport.
use ngspf::viewport;

/// `Port` used to display the viewport of NgsGameGFX.
#[derive(Debug)]
pub struct Port {}

impl viewport::Port for Port {
    fn mount(&self, objects: &viewport::GfxObjects) -> Box<viewport::PortInstance> {
        unimplemented!()
    }
}
