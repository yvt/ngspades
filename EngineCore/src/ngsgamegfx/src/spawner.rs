//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides a futures spawner for background tasks.
pub mod di {
    use injector::{prelude::*, Container};
    use std::sync::Arc;

    use super::*;

    pub trait SpawnerDeviceContainerExt {
        fn get_spawner(&self) -> Option<&Arc<dyn Spawner>>;
        fn get_spawner_or_build(&mut self) -> &Arc<dyn Spawner>;
        fn register_spawner_default(&mut self);
    }

    impl SpawnerDeviceContainerExt for Container {
        fn get_spawner(&self) -> Option<&Arc<dyn Spawner>> {
            self.get_singleton()
        }

        fn get_spawner_or_build(&mut self) -> &Arc<dyn Spawner> {
            self.get_singleton_or_build().unwrap()
        }

        fn register_spawner_default(&mut self) {
            self.register_singleton_factory(|_| -> Arc<dyn Spawner> {
                Arc::new(Queue::global(QueuePriority::Low))
            });
        }
    }
}

use xdispatch::{Queue, QueuePriority};

/// Trait for getting a spawner.
pub trait Spawner: Send + Sync + 'static + std::fmt::Debug {
    fn get_spawn<'a>(&'a self) -> Box<dyn futures::task::Spawn + 'a>;
}

impl Spawner for Queue {
    fn get_spawn<'a>(&'a self) -> Box<dyn futures::task::Spawn + 'a> {
        Box::new(self)
    }
}
