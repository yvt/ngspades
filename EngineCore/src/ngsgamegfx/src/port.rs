//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a NgsPF port type for embedding a NgsGameGFX viewport.
use ngspf::core::{
    Context, KeyedProperty, KeyedPropertyAccessor, PresenterFrame, PropertyAccessor,
};
use ngspf::viewport;
use std::sync::Arc;
#[allow(unused_imports)]
use zangfx::{base as gfx, prelude::*, utils as gfxut};

use config::Config;

/// `Port` used to display the viewport of NgsGameGFX.
#[derive(Debug, Clone)]
pub struct PortRef(Arc<PortProps>);

impl PortRef {
    pub fn new(context: &Context) -> Self {
        PortRef(Arc::new(PortProps::new(context)))
    }

    pub fn config<'a>(&'a self) -> impl PropertyAccessor<Config> + 'a {
        fn select(this: &Arc<PortProps>) -> &KeyedProperty<Config> {
            &this.config
        }
        KeyedPropertyAccessor::new(&self.0, select)
    }
}

impl viewport::Port for PortRef {
    fn mount(&self, objects: &viewport::GfxObjects) -> Box<viewport::PortInstance> {
        Box::new(Port {
            props: self.0.clone(),
            gfx_objects: objects.clone(),
            cb_state_tracker: None,
        })
    }
}

#[derive(Debug)]
struct PortProps {
    config: KeyedProperty<Config>,
}

impl PortProps {
    pub fn new(context: &Context) -> Self {
        Self {
            config: KeyedProperty::new(context, Config::default()),
        }
    }
}

/// The implementation of `PortInstance`.
#[derive(Debug)]
struct Port {
    props: Arc<PortProps>,
    gfx_objects: viewport::GfxObjects,
    cb_state_tracker: Option<gfxut::CbStateTracker>,
}

impl Drop for Port {
    fn drop(&mut self) {
        if let Some(x) = self.cb_state_tracker.take() {
            x.wait();
        }
    }
}

impl viewport::PortInstance for Port {
    fn image_extents(&self) -> [u32; 2] {
        [4, 4]
    }

    fn render(
        &mut self,
        context: &mut viewport::PortRenderContext,
        _frame: &PresenterFrame,
    ) -> gfx::Result<()> {
        // TODO: Render an actual content

        if let Some(x) = self.cb_state_tracker.take() {
            x.wait();
        }
        let mut cmd_buffer = self.gfx_objects.main_queue.queue.new_cmd_buffer()?;

        cmd_buffer.invalidate_image(&[&context.image]);
        {
            let enc = cmd_buffer.encode_copy();
            enc.update_fence(&context.fence, flags![gfx::AccessType::{}]);
        }

        self.cb_state_tracker = Some(gfxut::CbStateTracker::new(&mut *cmd_buffer));
        cmd_buffer.commit().unwrap();

        context.schedule_next_frame = true;
        Ok(())
    }
}
