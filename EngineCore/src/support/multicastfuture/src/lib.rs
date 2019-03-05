//! This crate provides a type for creating one or more `Future`s (the consuming
//! `Future`s) that can be used to observe the result of a provided `Future`
//! (the producing `Future`).
//!
//! The producing `Future` is evaluated only once. The consuming `Future`s can
//! be created or removed freely and dynamically while and/or after the
//! producing `Future` is evaluated. The consuming `Future`s must be polled
//! for the producing `Future` to be able to make a progress. `forget`ting
//! a consuming `Future` might indefinitely stall the progress.
//!
//! The result of the producing `Future` is broadcasted to the consuming
//! `Future` by `clone`-ing the result value. Therefore, the output type must
//! implement `Clone`.
//!
//! # Examples
//!
//! ```
//! #![feature(futures_api)]
//! use futures::{future::{lazy, FutureExt}, executor::block_on};
//! use multicastfuture::MultiCast;
//! use std::pin::Pin;
//!
//! let mut producer = lazy(|_| 42u32);
//!
//! let mc = MultiCast::new(producer);
//!
//! let consumer1 = Pin::new(&mc).subscribe();
//! let consumer2 = Pin::new(&mc).subscribe();
//!
//! assert_eq!(block_on(consumer1.join(consumer2)), (42, 42));
//! ```
//!
//! Make sure all consuming `Future`s are polled simultaneously. `MultiCast`
//! assumes that all live consumers are equally polled. The following code will
//! deadlock:
//!
//! ```no_run
//! # #![feature(futures_api)]
//! # use futures::{future::lazy, executor::block_on};
//! # use multicastfuture::MultiCast;
//! # use std::pin::Pin;
//! # let mut producer = lazy(|_| 42u32);
//! # let mc = MultiCast::new(producer);
//! let _consumer1 = Pin::new(&mc).subscribe();
//! let consumer2 = Pin::new(&mc).subscribe();
//!
//! block_on(consumer2);
//! ```
//!
//! Just make sure to drop unused consumers:
//!
//! ```
//! # #![feature(futures_api)]
//! # use futures::{future::lazy, executor::block_on};
//! # use multicastfuture::MultiCast;
//! # use std::pin::Pin;
//! # let mut producer = lazy(|_| 42u32);
//! # let mc = MultiCast::new(producer);
//! let consumer1 = Pin::new(&mc).subscribe();
//! let consumer2 = Pin::new(&mc).subscribe();
//!
//! drop(consumer1);
//! block_on(consumer2);
//! ```
//!
#![feature(arbitrary_self_types)]
#![feature(futures_api)]
#![feature(maybe_uninit)]
#![feature(maybe_uninit_ref)]
use futures::{ready, task::Waker, Future, Poll};
use parking_lot::Mutex;
use std::{
    cell::UnsafeCell,
    fmt,
    mem::MaybeUninit,
    ops::Deref,
    pin::Pin,
    ptr::null_mut,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

/// Broadcasts the result of a `Future` (the producing `Future`) to one or more
/// `Future`s (the consuming `Future`s).
///
/// See [the crate documentation](index.html) for details.
pub struct MultiCast<F: Future> {
    /// The producing `Future`. Only can be accessed by a leader.
    future: UnsafeCell<F>,

    /// The result cell.
    result: UnsafeCell<MaybeUninit<F::Output>>,

    /// The pointer to a consumer's `ConsumerState` which is responsible for
    /// polling the producing `Future`. `null` indicates there's no consumer.
    ///
    /// The modification to this field is protected by `MultiCast::mutex`.
    ///
    /// This field becomes `null` after the completion of
    /// the producing `Future`.
    leader: AtomicPtr<ConsumerState>,

    /// Indicates whether the producing `Future` (`MultiCast::future`) has been
    /// completed or not.
    complete: AtomicBool,

    /// The mutex for protecting the state of the consumer list.
    mutex: Mutex<()>,
}

/// The consuming `Future` of [`MultiCast`].
///
/// See [the crate documentation](index.html) for details.
#[derive(Debug)]
pub struct Consumer<P: Deref<Target = MultiCast<F>>, F: Future> {
    producer: Pin<P>,
    state: Option<Pin<Box<ConsumerState>>>,
}

/// The state of a consumer.
///
/// This must be a separate struct from `Consumer` because `Consumer` can vanish
/// anytime through the use of `std::mem::forget`.
#[derive(Debug, Default)]
struct ConsumerState {
    /// The waker used in the following situations:
    ///
    ///  - This consumer receives a leadership (i.e, being assigned to
    ///    `MultiCast::leader`).
    ///  - The completion of the producing `Future`.
    ///
    task: Mutex<Option<Waker>>,

    /// The pointers to the previous and next `ConsumerState`s in a circular
    /// linked list.
    ///
    /// The modification to this field is protected by `MultiCast::mutex`.
    prev_next: [AtomicPtr<ConsumerState>; 2],
}

impl<F: Future> MultiCast<F> {
    /// Construct a `MultiCast` by wrapping a given `Future`.
    pub fn new(inner: F) -> Self {
        Self {
            future: UnsafeCell::new(inner),
            result: UnsafeCell::new(MaybeUninit::uninitialized()),
            leader: AtomicPtr::default(),
            complete: AtomicBool::new(false),
            mutex: Mutex::new(()),
        }
    }

    /// Create a consuming `Future`.
    pub fn subscribe<P: Deref<Target = Self>>(self: Pin<P>) -> Consumer<P, F> {
        let state = loop {
            let this = &*self;
            let _lock = this.mutex.lock();

            if this.complete.load(Ordering::Relaxed) {
                break None;
            }

            // Insert the consumer into the list
            let mut state = Box::pin(ConsumerState::default());
            let state_ptr = (&*state) as *const _ as *mut _;

            let leader = this.leader.load(Ordering::Acquire);
            if leader.is_null() {
                this.leader
                    .store((&*state) as *const _ as *mut _, Ordering::Relaxed);

                *state.prev_next[0].get_mut() = state_ptr;
                *state.prev_next[1].get_mut() = state_ptr;
            } else {
                unsafe {
                    let (prev, next) = (leader, (&*leader).prev_next[1].load(Ordering::Relaxed));

                    *state.prev_next[0].get_mut() = prev;
                    *state.prev_next[1].get_mut() = next;

                    (&*prev).prev_next[1].store(state_ptr, Ordering::Relaxed);
                    (&*next).prev_next[0].store(state_ptr, Ordering::Relaxed);
                }
            }

            break Some(state);
        };

        Consumer {
            producer: self,
            state,
        }
    }

    /// Check if the result is ready.
    pub fn is_complete(&self) -> bool {
        self.complete.load(Ordering::Relaxed)
    }

    /// Get a reference to the result if it's ready.
    pub fn result(&self) -> Option<&F::Output> {
        if self.complete.load(Ordering::Acquire) {
            unsafe { Some((&*self.result.get()).get_ref()) }
        } else {
            None
        }
    }

    /// Get a mutable reference to the result if it's ready.
    pub fn result_mut(&mut self) -> Option<&mut F::Output> {
        if *self.complete.get_mut() {
            unsafe { Some((&mut *self.result.get()).get_mut()) }
        } else {
            None
        }
    }

    /// Attempt to get the result. Returns the original object if the result is
    /// is not ready yet.
    pub fn try_into_result(mut self) -> Result<F::Output, Self> {
        if *self.complete.get_mut() {
            *self.complete.get_mut() = false; // Suppress `drop`
            unsafe { Ok((&*self.result.get()).as_ptr().read()) }
        } else {
            Err(self)
        }
    }
}

impl<F: Future> Drop for MultiCast<F> {
    fn drop(&mut self) {
        if *self.complete.get_mut() {
            unsafe {
                (&mut *self.result.get()).as_mut_ptr().drop_in_place();
            }
        }
    }
}

unsafe impl<F: Future> Sync for MultiCast<F>
where
    F: Sync,
    F::Output: Sync,
{
}

impl<F: Future> fmt::Debug for MultiCast<F>
where
    F: fmt::Debug,
    F::Output: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.complete.load(Ordering::Acquire) {
            f.debug_struct("MultiCast")
                .field("future", unsafe { &*self.future.get() })
                .field("result", self.result().unwrap())
                .field("complete", &true)
                .finish()
        } else {
            f.debug_struct("MultiCast")
                .field("complete", &false)
                .finish()
        }
    }
}

impl<P: Deref<Target = MultiCast<F>>, F: Future> Consumer<P, F> {
    /// Get the original reference to [`MultiCast`].
    pub fn multi_cast(&self) -> &Pin<P> {
        &self.producer
    }
}

impl<P: Deref<Target = MultiCast<F>>, F: Future> Future for Consumer<P, F>
where
    F::Output: Clone,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, waker: &Waker) -> Poll<Self::Output> {
        let this = &*self;
        let producer = &*this.producer;
        if let Some(state) = &this.state {
            let state_ptr: *mut ConsumerState = (&**state) as *const _ as *mut _;

            if producer.complete.load(Ordering::Acquire) {
                // We already have the result
            } else if producer.leader.load(Ordering::Acquire) == state_ptr {
                // This consumer is responsible for polling the producing `Future`.

                // `&mut *producer.future.get()` because this consumer is the
                // current leader.
                // `Pin::new_unchecked` is safe here because we do not move the
                // contents of `MultiCast::future` once `Pin<P>` started
                // existing and `MultiCast` itself is pinned by `Pin<P>`.
                let inner = unsafe { Pin::new_unchecked(&mut *producer.future.get()) };

                // Poll the future
                let value = ready!(inner.poll(waker));

                // Store the result and wake up all consumers (except `self`)
                let _lock = producer.mutex.lock();
                unsafe {
                    (&mut *producer.result.get()).set(value);
                    producer.complete.store(true, Ordering::Release);

                    let mut ptr = state.prev_next[1].load(Ordering::Relaxed);
                    while ptr != state_ptr {
                        let other_state = &*ptr;
                        if let Some(waker) = &*other_state.task.lock() {
                            waker.wake();
                        }
                        ptr = other_state.prev_next[1].load(Ordering::Relaxed);
                    }
                }
            } else {
                // Register the waker
                let mut waker_cell = state.task.lock();

                if waker_cell.as_ref().map(|w| w.will_wake(waker)) != Some(true) {
                    *waker_cell = Some(Waker::clone(waker));
                }

                return Poll::Pending;
            }
        } else {
            // The `Future` was already complete at the point when `subscribe`
            // was called
        }

        let value = unsafe { (&*producer.result.get()).get_ref().clone() };
        Poll::Ready(value)
    }
}

impl<P: Deref<Target = MultiCast<F>>, F: Future> Drop for Consumer<P, F> {
    fn drop(&mut self) {
        if let Some(state) = &self.state {
            let producer = &*self.producer;

            let state_ptr: *mut ConsumerState = (&**state) as *const _ as *mut _;

            let _lock = producer.mutex.lock();

            if producer.complete.load(Ordering::Relaxed) {
                return;
            }

            // If this consumer is the current leader, transfer the leadership
            // to another consumer
            if producer.leader.load(Ordering::Relaxed) == state_ptr {
                let new_leader = state.prev_next[1].load(Ordering::Relaxed);
                if new_leader == state_ptr {
                    // The list is now empty.
                    producer.leader.store(null_mut(), Ordering::Release);

                    return;
                } else {
                    producer.leader.store(new_leader, Ordering::Release);

                    // Wake up the new leader so that the producing `Future`
                    // knows which `Waker` to wake up next
                    if let Some(waker) = &*(unsafe { &*new_leader }.task.lock()) {
                        waker.wake();
                    }
                }
            }

            // Remove this consumer from the list
            unsafe {
                let prev = state.prev_next[0].load(Ordering::Relaxed);
                let next = state.prev_next[1].load(Ordering::Relaxed);

                debug_assert_ne!(prev, state_ptr);
                debug_assert_ne!(next, state_ptr);

                (&*prev).prev_next[1].store(next, Ordering::Relaxed);
                (&*next).prev_next[0].store(prev, Ordering::Relaxed);
            }
        }
    }
}
