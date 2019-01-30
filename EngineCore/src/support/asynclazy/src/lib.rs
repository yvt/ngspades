//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Provides the asynchonously evaluated cell type analogous to
//! `std::async(std::launch::async, ...)` from C++.
#![feature(futures_api)]
#![feature(box_patterns)]
use atom2::SetOnceAtom;
use futures::{
    prelude::*,
    task::{Spawn, SpawnError, SpawnExt},
};
use parking_lot::Mutex;
use std::sync::mpsc;

/// An aynchronously evaluated cell.
#[derive(Debug)]
pub struct Async<T> {
    /// A channel for receiving an evaluted value. The value will be
    /// moved to `inner` as soon as its reception
    initer: Mutex<mpsc::Receiver<T>>,
    /// Stores an evaluated value.
    /// This cell only can be assigned while `initer` is locked.
    inner: SetOnceAtom<Box<T>>,
}

impl<T: Send + 'static> Async<T> {
    /// Construct a `Async`. A given `Future` is spawned using a given `spawner`
    /// to compute the cell's value.
    ///
    /// Note that the future is *not* terminated if `Async` is dropped
    /// prematurely.
    pub fn with_future(
        spawner: &mut (impl Spawn + ?Sized),
        value: impl Future<Output = T> + Send + 'static,
    ) -> Result<Self, SpawnError> {
        let (send, recv) = mpsc::sync_channel(1);

        spawner.spawn(value.map(move |result| {
            drop(send.send(result));
        }))?;

        Ok(Self {
            initer: Mutex::new(recv),
            inner: SetOnceAtom::empty(),
        })
    }
}

impl<T> Async<T> {
    /// Construct an initialized `Async`.
    pub fn with_value(x: T) -> Self {
        let (_, recv) = mpsc::sync_channel(0);
        Self {
            initer: Mutex::new(recv),
            inner: SetOnceAtom::new(Some(Box::new(x))),
        }
    }

    fn check_blocking(&self) {
        // Is it already initialized?
        if !self.inner.get().is_none() {
            return;
        }

        let initer = self.initer.lock();

        // Check it again because `check` might have been called
        // in another thread since we checked it
        if !self.inner.get().is_none() {
            return;
        }

        // Wait for the result
        let result = initer.recv().expect("sending end dropped unexpectedly");

        match self.inner.store(Some(Box::new(result))) {
            Ok(()) => {}
            Err(_) => unreachable!(),
        }
    }

    fn check_nonblocking(&self) {
        // Is it already initialized?
        if !self.inner.get().is_none() {
            return;
        }

        let initer = if let Some(x) = self.initer.try_lock() {
            x
        } else {
            // Another thread is being blocked - this means the result is
            // unavailable yet
            return;
        };

        // Check it again because `check` might have been called
        // in another thread since we checked it
        if !self.inner.get().is_none() {
            return;
        }

        // Check the availability
        let result = if let Ok(x) = initer.try_recv() {
            x
        } else {
            // The result is unavailable yet
            return;
        };

        match self.inner.store(Some(Box::new(result))) {
            Ok(()) => {}
            Err(_) => unreachable!(),
        }
    }

    /// Get a reference to an evaluated value. Blocks the current thread until
    /// the value is available.
    pub fn get(&self) -> &T {
        // FIXME: Ideally this could call `try_get` first to avoid the overhead
        //        due to loading `self.inner` twice but the borrow checker
        //        wasn't happy about it:
        //        <https://github.com/rust-lang/rust/issues/54663>
        self.check_blocking();
        self.inner.as_inner_ref().unwrap()
    }

    /// Get a reference to an evaluated value. Returns `None` if the value
    /// is not available at the point when the method is called.
    pub fn try_get(&self) -> Option<&T> {
        self.check_nonblocking();
        self.inner.as_inner_ref()
    }

    /// Get a mutable reference to an evaluated value. Blocks the current thread
    /// until the value is available.
    pub fn get_mut(&mut self) -> &mut T {
        self.check_blocking();
        self.inner.as_inner_mut().unwrap()
    }

    /// Get a mutable reference to an evaluated value. Returns `None` if the
    /// value is not available at the point when the method is called.
    pub fn try_get_mut(&mut self) -> Option<&mut T> {
        self.check_nonblocking();
        self.inner.as_inner_mut()
    }

    /// Consume `Self`, returning an evaluated value. Blocks the current thread
    /// until the value is available.
    pub fn into_inner(self) -> T {
        self.check_blocking();

        let box x = self.inner.into_inner().unwrap();
        x
    }

    /// Consume `Self`, returning an evaluated value. Returns `Err(self)` if the
    /// value is not available at the point when the method is called.
    pub fn try_into_inner(self) -> Result<T, Self> {
        self.check_nonblocking();

        if let Some(box x) = self.inner.into_inner() {
            Ok(x)
        } else {
            Err(Self {
                initer: self.initer,
                inner: SetOnceAtom::empty(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::{channel::oneshot, executor::ThreadPool};
    use std::{sync::Arc, thread, time::Duration};

    #[test]
    fn sync() {
        assert_eq!(*Async::with_value(42).get(), 42);
        assert_eq!(*Async::with_value(42).get_mut(), 42);
        assert_eq!(Async::with_value(42).try_get().cloned(), Some(42));
        assert_eq!(Async::with_value(42).try_get_mut().cloned(), Some(42));
        assert_eq!(Async::with_value(42).into_inner(), 42);
        assert_eq!(Async::with_value(42).try_into_inner().unwrap(), 42);
    }

    #[test]
    fn futures() {
        let (send, recv) = oneshot::channel();

        let pool = Arc::new(ThreadPool::new().unwrap());

        // Start a new thread where the evaluated value is observed
        let handle = {
            let pool = Arc::clone(&pool);

            thread::Builder::new()
                .spawn(move || {
                    let fut = recv.map(|x| x.unwrap());
                    let a = Async::with_future(&mut &*pool, fut).unwrap();

                    // The result is still unevaluated
                    assert_eq!(a.try_get().cloned(), None);

                    let a = match a.try_into_inner() {
                        Ok(_) => unreachable!(),
                        Err(a) => a,
                    };

                    // Wait for the result
                    assert_eq!(*a.get(), 42);
                })
                .unwrap()
        };

        thread::sleep(Duration::from_millis(100));

        // Complete computation
        send.send(42).unwrap();

        handle.join().unwrap();
    }
}
