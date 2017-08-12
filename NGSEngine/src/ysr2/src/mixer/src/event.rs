//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use parking_lot::{Mutex, Condvar};
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Event(Arc<EventData>);

struct EventData {
    mutex: Mutex<bool>,
    cond: Condvar,
}

impl fmt::Debug for EventData {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("EventData")
            .field(&*self.mutex.lock())
            .finish()
    }
}

impl Event {
    pub fn new() -> Self {
        Event(Arc::new(EventData {
            mutex: Mutex::new(false),
            cond: Condvar::new(),
        }))
    }

    pub fn wait(&self) {
        let ref data = self.0;
        let mut lock = data.mutex.lock();
        while !*lock {
            data.cond.wait(&mut lock);
        }
    }

    pub fn reset(&self) {
        *self.0.mutex.lock() = false;
    }

    pub fn set(&self) {
        let ref data = self.0;
        let mut lock = data.mutex.lock();
        *lock = true;
        data.cond.notify_all();
    }

    pub fn is_set(&self) -> bool {
        *self.0.mutex.lock()
    }
}
