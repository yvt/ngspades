//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use winit::{EventsLoopProxy, Window};
use zangfx::backends::metal::metal;

use zangfx::base as gfx;
use zangfx::backends::metal as be;

use metalutils::OCPtr;
use super::{Painter, WmDevice};

pub fn autorelease_pool_scope<T, S>(cb: T) -> S
where
    T: FnOnce(&mut AutoreleasePool) -> S,
{
    let mut op = AutoreleasePool(Some(unsafe {
        OCPtr::from_raw(metal::NSAutoreleasePool::alloc().init()).unwrap()
    }));
    cb(&mut op)
}

pub struct AutoreleasePool(Option<OCPtr<metal::NSAutoreleasePool>>);

impl AutoreleasePool {
    pub fn drain(&mut self) {
        self.0 = None;
        self.0 =
            Some(unsafe { OCPtr::from_raw(metal::NSAutoreleasePool::alloc().init()).unwrap() });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceRef(u32);

pub struct WindowManager<P> {
    painter: P,
    events_loop_proxy: EventsLoopProxy,
}

impl<P> ::Debug for WindowManager<P>
where
    P: ::Debug,
{
    fn fmt(&self, fmt: &mut ::fmt::Formatter) -> ::fmt::Result {
        fmt.debug_struct("WindowManager")
            .field("painter", &self.painter)
            .finish()
    }
}

impl<P: Painter> WindowManager<P> {
    pub fn new(painter: P, events_loop_proxy: EventsLoopProxy) -> Self {
        Self {
            painter,
            events_loop_proxy,
        }
    }

    pub fn painter_ref(&self) -> &P {
        &self.painter
    }

    pub fn painter_mut(&mut self) -> &mut P {
        &mut self.painter
    }

    pub fn add_surface(&mut self, window: &Window, param: P::SurfaceParam) -> SurfaceRef {
        unimplemented!()
    }

    pub fn remove_surface(&mut self, surface_ref: SurfaceRef) {
        unimplemented!()
    }

    pub fn update(&mut self) {
        unimplemented!()
    }
}
