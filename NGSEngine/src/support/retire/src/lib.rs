//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Building blocks for manageing lifetimes of and accesses to objects shared
//! with regards to access by external entities (e.g., GPU, remote system).
//!
//! ## System Model
//!
//!  - A **host** is where your Rust program runs. All **resources** are created
//!    here.
//!  - A **queue** receives and processes commands submitted by a host.
//!    Commands are processed by a **device** (an external entity that consumes
//!    commands and resources) in FIFO fashion and may include references to
//!    resources.
//!
//! A host can perform the following operations with a queue:
//!
//!  - **Submit**: submit one or more commands to a queue.
//!  - **Fence**: insert a **fence** that will be signaled upon the completion
//!    of all preceding commands submitted to the queue (by the *Submit*
//!    operation). This definition leads to the following observations:
//!      - All fences inserted to the same queue have a total chronological
//!        ordering.
//!      - A signaled fence implies all preceding fences are signaled.
//!  - **Wait**: block the execution of the current thread until a given *fence*
//!    is signaled.
//!  - **Query**: query whether a given *fence* was signaled or not.
//!
//! ## Examples
//!
//! ```
//! use std::sync::{Arc, Mutex};
//! use std::time::Duration;
//! use retire::*;
//! use retire::tokenlock::{Token, TokenRef};
//!
//! struct MyQueue {
//!     token: Mutex<Token>,
//!     token_ref: TokenRef,
//! }
//!
//! impl MyQueue {
//!     fn new() -> Self {
//!         let token = Token::new();
//!         Self {
//!             token_ref: (&token).into(),
//!             token: Mutex::new(token),
//!         }
//!     }
//! }
//!
//! struct MyFence {
//!     table: Mutex<Option<RefTable<ResourceData>>>,
//! }
//! impl Fence for MyFence {
//!     fn wait_timeout(&self, _timeout: Duration) -> bool {
//!         self.table.lock().unwrap().take();
//!         true
//!     }
//! }
//!
//! #[derive(PartialEq, Eq, Hash, Clone)]
//! struct ResourceData(Arc<()>);
//! impl Res for ResourceData {}
//!
//! struct Resource {
//!     data: ResourceData,
//!     queue_data: ResQueueData<MyFence>,
//!     queue: Arc<MyQueue>,
//! }
//!
//! impl Resource {
//!     fn new(queue: Arc<MyQueue>) -> Self {
//!         Self {
//!             data: ResourceData(Arc::new(())),
//!             queue_data: ResQueueData::new(queue.token_ref.clone()),
//!             queue,
//!         }
//!     }
//!
//!     fn lock_host_access(&self) -> Option<&()> {
//!         if Arc::strong_count(&self.data.0) > 1 {
//!             let ref queue = self.queue;
//!             if !self.queue_data.wait_timeout(
//!                 |x| x(&queue.token.lock().unwrap()),
//!                 Duration::new(0, 0),
//!             ).unwrap() {
//!                 return None;
//!             }
//!         }
//!         if Arc::strong_count(&self.data.0) > 1 {
//!             None
//!         } else {
//!             Some(&self.data.0)
//!         }
//!     }
//! }
//!
//! fn submit_command(queue: &MyQueue, used_resource: &Resource) -> Arc<MyFence> {
//!     let mut token = queue.token.lock().unwrap();
//!     let mut builder = RefTableBuilder::new();
//!     builder.insert(used_resource.data.clone());
//!     let fence = Arc::new(MyFence {
//!         table: Mutex::new(Some(builder.build())),
//!     });
//!     used_resource.queue_data.associate(&mut token, fence.clone());
//!     fence
//! }
//!
//! let queue = Arc::new(MyQueue::new());
//! let mut res = Resource::new(queue.clone());
//! submit_command(&queue, &mut res);
//!
//! assert!(res.lock_host_access().is_some());
//! ```
pub extern crate tokenlock;
use std::sync::Arc;
use std::hash::Hash;
use std::time::Duration;
use std::collections::HashSet;
use tokenlock::{TokenRef, TokenLock, Token};

pub struct ResQueueData<F> {
    fence: TokenLock<Option<Arc<F>>>,
}

impl<F: Fence> ResQueueData<F> {
    pub fn new<T: Into<TokenRef>>(token_ref: T) -> Self {
        Self { fence: TokenLock::new(token_ref, None) }
    }

    /// Set the latest fence that references the resource.
    ///
    /// Returns `Err(())` if the token does not match the one given at creation
    /// time.
    pub fn associate(&self, token: &mut Token, fence: Arc<F>) -> Result<(), ()> {
        *self.fence.write(token).ok_or(())? = Some(fence);
        Ok(())
    }

    /// Block the current thread until the latest fence is signaled, timing out
    /// after a specified duration.
    ///
    /// Returns `Err(())` if the token does not match the one given at creation
    /// time.
    pub fn wait_timeout<T>(&self, acquire_lock: T, timeout: Duration) -> Result<bool, ()>
    where
        T: FnOnce(&Fn(&Token) -> ResQueueDataWaitContext<F>) -> ResQueueDataWaitContext<F>,
    {
        let fence = acquire_lock(&|token| if let Some(fence) = self.fence.read(token) {
            ResQueueDataWaitContext(Ok(fence.clone()))
        } else {
            ResQueueDataWaitContext(Err(()))
        }).0?;
        if let Some(ref fence) = fence {
            Ok(fence.wait_timeout(timeout))
        } else {
            Ok(false)
        }
    }
}

pub struct ResQueueDataWaitContext<F>(Result<Option<Arc<F>>, ()>);

pub struct RefTable<T> {
    set: HashSet<T>,
}

impl<T: Res> RefTable<T> {
    /// Clear the references to the resources.
    ///
    /// Returns a fresh, empty `RefTableBuilder`.
    pub fn retire(mut self) -> RefTableBuilder<T> {
        self.set.clear();

        RefTableBuilder { set: self.set }
    }
}

pub struct RefTableBuilder<T> {
    set: HashSet<T>,
}

impl<T: Res> RefTableBuilder<T> {
    pub fn new() -> Self {
        Self { set: HashSet::new() }
    }

    pub fn insert(&mut self, x: T) {
        self.set.insert(x);
    }

    pub fn build(self) -> RefTable<T> {
        RefTable { set: self.set }
    }
}

/// Resource data type.
///
/// `Res` is stored to reference tables (`RefTable`), and the lifetime of the
/// stored `Res` is at least as long as the duration for which the resource is
/// possibly used by a device.
pub trait Res: PartialEq + Eq + Hash {}

pub trait Fence {
    /// Block the current thread until the fence is signaled, timing out after
    /// a specified duration.
    ///
    /// Calls `retire()` on all contained `RefTable`s, or just drops them before
    /// returning if the fence was signaled within the specified time out
    /// duration.
    fn wait_timeout(&self, timeout: Duration) -> bool;
}
