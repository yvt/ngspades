//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a NgsPF port type for embedding a NgsGameGFX viewport.
use ngspf::core::{KeyedProperty, KeyedPropertyAccessor, PropertyAccessor};
use ngspf::viewport;
use std::sync::Arc;

use config::Config;

/// `Port` used to display the viewport of NgsGameGFX.
#[derive(Debug, Clone)]
pub struct PortRef(Arc<Port>);

impl PortRef {
    pub fn config<'a>(&'a self) -> impl PropertyAccessor<Config> + 'a {
        fn select(this: &Arc<Port>) -> &KeyedProperty<Config> {
            &this.config
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }
}

impl viewport::Port for PortRef {
    fn mount(&self, _objects: &viewport::GfxObjects) -> Box<viewport::PortInstance> {
        unimplemented!()
    }
}

#[derive(Debug)]
struct Port {
    config: KeyedProperty<Config>,
}
