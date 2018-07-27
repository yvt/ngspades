//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Volatile access views.
//!
//! This crate provides the [`Volatile<T>`](Volatile) type representing a memory
//! region where volatile reads and writes can be performed without
//! causing "undefined behaviors". Volatile access views represented by
//! `&Volatile<T>`s and `&[Volatile<T>]`s can be divided or reinterpreted
//! (via [`VolatilePod`] or [`VolatileSlicePod`]), or even be
//! shared among threads.
//!
//! Volatile reads and writes are directly translated into native load and store
//! instructions of the target architecture. Furthermore, `Volatile<T>`
//! prohibits formation of a reference to the contained value, acting like a
//! `Cell<T>`, which precludes the compiler from making various assumptions that
//! only can be made for normal references and pointers. Therefore,
//! `Volatile<T>` can be used safely, assuming that the target architecture
//! does not [behave] in a weird way.
//!
//! [behave]: https://llvm.org/docs/LangRef.html#memory-model-for-concurrent-operations
//!
//! A volatile access view is constructed by one of the following methods:
//!
//!  - [`Volatile::from_mut`] converts a mutable reference of type `&mut T` into
//!    a volatile access view of type `&Volatile<T>`. This is safe in most cases
//!    since accessibility via non-volatile memory access usually implies that
//!    via volatile memory access and the possession of a mutable reference
//!    indicates the former.
//!
//!  - [`Volatile::from_raw`] convert a raw pointer of type `*mut T` into a
//!    volatile access view of type `&Volatile<T>`. These are unsafe for
//!    obvious reasons.
//!
//!  - [`Volatile::new`] constructs a volatile-accessed cell on the stack.
//!
//! # Prior art
//!
//! [`volatile`], [`volatile-register`], and [`volatile_cell`] all provide
//! wrapper cell types for volatile accesses similarly to `volatile_view`,
//! except that none of them support the slicing, dividing, and reinterpreting
//! operations on a volatile reference as they don't impose the POD restriction
//! on the contained data. Some of them implement read/write access control
//! which `volatile_view` doesn't.
//!
//! Some of the existing crates are unsound because they allow wrapping any
//! `Copy` types (not just POD types). Simultaneous access to a volatile
//! variable of a non-POD type from multiple threads is not safe because the
//! contents might enter an invalid state because of memory tearing. The others
//! require at least one call to an `unsafe` function to do such operations.
//!
//! [`volatile-ptr`] takes a different approach and provides
//! a volatile *pointer* type.
//!
//! [`volatile`]: https://crates.io/crates/volatile
//! [`volatile-register`]: https://crates.io/crates/volatile-register
//! [`volatile_cell`]: https://crates.io/crates/volatile_cell
//! [`volatile-ptr`]: https://crates.io/crates/volatile-ptr
extern crate pod;

use pod::Pod;
use std::{cell::UnsafeCell, fmt, mem::transmute};

/// A volatile access view.
///
/// See [the crate documentation](index.html) for a general description about
/// volatile access views.
///
pub struct Volatile<T>(UnsafeCell<T>);

unsafe impl<T: Send> Send for Volatile<T> {}
unsafe impl<T: Sync> Sync for Volatile<T> {}

impl<T> Volatile<T> {
    /// Construct a volatile access view from a mutable reference.
    ///
    /// This is safe because the possession of a mutable reference indicates
    /// that there exists no way to simultaneously access the same contents via
    /// normal memory access.
    ///
    /// # Examples
    ///
    ///     # use volatile_view::*;
    ///     let mut x = 5;
    ///     let view: &Volatile<u32> = Volatile::from_mut(&mut x);
    ///
    pub fn from_mut<'a>(x: &'a mut T) -> &'a mut Self {
        unsafe { &mut *(Self::from_ref(x) as *const _ as *mut _) }
    }

    /// `&T` version of `from_mut`. We use this to not accidentally form mutable
    /// references.
    unsafe fn from_ref<'a>(x: &'a T) -> &'a Self {
        transmute(x)
    }

    /// Construct a slice of volatile access views from a mutable reference.
    ///
    /// This is safe because the possession of a mutable reference indicates
    /// that there exists no way to simultaneously access the same contents via
    /// normal memory access.
    ///
    /// # Examples
    ///
    ///     # use volatile_view::*;
    ///     let mut x = [0u32; 4];
    ///     let view: &[Volatile<u32>] = Volatile::slice_from_mut(&mut x[..]);
    ///
    pub fn slice_from_mut<'a>(x: &'a mut [T]) -> &'a mut [Self] {
        unsafe { &mut *(Self::slice_from_ref(x) as *const _ as *mut _) }
    }

    /// `&[T]` version of `slice_from_mut`. We use this to not accidentally form
    ///  mutable references.
    unsafe fn slice_from_ref<'a>(x: &'a [T]) -> &'a [Self] {
        transmute(x)
    }

    /// Construct a volatile access view from a raw pointer.
    ///
    /// `x` must be non-null.
    pub unsafe fn from_raw(x: *mut T) -> &'static Self {
        Self::from_ref(&*x)
    }

    /// Construct a slice of volatile access views from a raw pointer.
    ///
    /// `x` must be non-null.
    pub unsafe fn slice_from_raw(x: *mut T, len: usize) -> &'static [Self] {
        Self::slice_from_ref(::std::slice::from_raw_parts(x as *const _, len))
    }

    /// Construct a cell accessed via a volatile access view.
    pub fn new(x: T) -> Self {
        Volatile(UnsafeCell::new(x))
    }

    /// Unwrap the cell.
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }

    /// Get a mutable reference to the inner value.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.as_ptr() }
    }

    /// Get a raw pointer to the inner value.
    pub fn as_ptr(&self) -> *mut T {
        self.0.get()
    }

    /// Convert `&self` to a `&T`. The contents must not be accessed via
    /// standard means.
    unsafe fn as_ref(&self) -> &T {
        &*self.as_ptr()
    }

    /// Load a value from `self` even if `T` is not a POD type.
    pub unsafe fn load_unchecked(&self) -> T {
        self.as_ptr().read_volatile()
    }

    /// Store a value to `self` even if `T` is not a POD type.
    pub unsafe fn store_unchecked(&self, x: T) {
        self.as_ptr().write_volatile(x);
    }
}

impl<T: Pod> Volatile<T> {
    /// Load a value from `self`.
    pub fn load(&self) -> T {
        unsafe { self.load_unchecked() }
    }

    /// Store a value to `self`.
    pub fn store(&self, x: T) {
        unsafe { self.store_unchecked(x) }
    }
}

/// Extensions of the [`Pod`](../pod/trait.Pod.html) trait for [`Volatile`]`<T>`.
pub trait VolatilePod {
    /// Convert a volatile reference from one to another type of the same size.
    ///
    /// This method reflects the functionality of `pod::Pod::map`.
    ///
    /// Returns `None` if the source and destination types are misaligned or
    /// not the same size.
    ///
    /// # Examples
    ///
    ///     # use volatile_view::*;
    ///     let x: Volatile<f32> = Volatile::new(42.0f32);
    ///
    ///     // Transmute the view to `u32`
    ///     let x_view_u32: &Volatile<u32> = x.map().unwrap();
    ///
    ///     // Both of the results of transmutations should be identical
    ///     assert_eq!(x_view_u32.load(), x.load().to_bits());
    ///
    fn map<U: Pod>(&self) -> Option<&Volatile<U>>;

    /// Split a volatile reference from one to a slice of another type.
    ///
    /// This method reflects the functionality of `pod::Pod::split`.
    ///
    /// Returns `None` if the source and destination types are misaligned or
    /// the source does not fit perfectly in the destination slice type.
    ///
    /// # Examples
    ///
    ///     # use volatile_view::*;
    ///     let mut x: Volatile<u32> = Volatile::new(0x42424242u32);
    ///
    ///     // Split the view into `u8`s
    ///     let x_bytes: &[Volatile<u8>] = x.split().unwrap();
    ///
    ///     assert_eq!(x_bytes[0].load(), 0x42);
    ///     assert_eq!(x_bytes[1].load(), 0x42);
    ///     assert_eq!(x_bytes[2].load(), 0x42);
    ///     assert_eq!(x_bytes[3].load(), 0x42);
    ///
    fn split<U: Pod>(&self) -> Option<&[Volatile<U>]>;
}

impl<T: Pod> VolatilePod for Volatile<T> {
    fn map<U: Pod>(&self) -> Option<&Volatile<U>> {
        unsafe { Pod::map(self.as_ref()).map(|x| Volatile::from_ref(x)) }
    }

    fn split<U: Pod>(&self) -> Option<&[Volatile<U>]> {
        unsafe { Pod::split(self.as_ref()).map(|x| Volatile::slice_from_ref(x)) }
    }
}

/// Extensions of the [`Pod`](../pod/trait.Pod.html) trait for `[`[`Volatile`]`<T>]`.
pub trait VolatileSlicePod<T> {
    /// Convert a volatile slice reference from one to another type.
    ///
    /// This method reflects the functionality of `pod::Pod::map_slice`.
    ///
    /// Returns `None` if the source and destination types are misaligned or
    /// the source does not fit perfectly in the destination slice type.
    ///
    /// # Examples
    ///
    ///     # use volatile_view::*;
    ///     let mut x = [0x4242u16; 2];
    ///     let x_view: &[Volatile<u16>] = Volatile::slice_from_mut(&mut x[..]);
    ///
    ///     // Split the views into four `u8`s
    ///     let x_bytes: &[Volatile<u8>] = x_view.map_slice().unwrap();
    ///
    ///     assert_eq!(x_bytes[0].load(), 0x42);
    ///     assert_eq!(x_bytes[1].load(), 0x42);
    ///     assert_eq!(x_bytes[2].load(), 0x42);
    ///     assert_eq!(x_bytes[3].load(), 0x42);
    ///
    fn map_slice<U: Pod>(&self) -> Option<&[Volatile<U>]>;

    /// Convert a volatile slice reference to another type.
    ///
    /// This method reflects the functionality of `pod::Pod::merge`.
    ///
    /// Returns `None` if the source and destination types are misaligned or
    /// not the same size.
    ///
    /// # Examples
    ///
    ///     # use volatile_view::*;
    ///     let mut x = [0x42u8; 4];
    ///     let x_view: &[Volatile<u8>] = Volatile::slice_from_mut(&mut x[..]);
    ///
    ///     // Marge the views into one `u32`
    ///     let x_merged: &Volatile<u32> = x_view.merge().unwrap();
    ///
    ///     assert_eq!(x_merged.load(), 0x42424242);
    ///
    fn merge<U: Pod>(&self) -> Option<&Volatile<U>>;

    /// Copy all elements to `slice`.
    ///
    /// # Panics
    ///
    /// This function will panic if `slice.len() != self.len()`.
    fn copy_to_slice(&self, slice: &mut [T]);

    /// Copy all elements from `slice`.
    ///
    /// # Panics
    ///
    /// This function will panic if `slice.len() != self.len()`.
    fn copy_from_slice(&self, slice: &[T]);
}

/// Convert `&[Volatile<T>]` to a `&[T]`. The contents must not be accessed via
/// standard means.
unsafe fn unwrap_as_slice<T>(x: &[Volatile<T>]) -> &[T] {
    transmute(x)
}

impl<T: Pod> VolatileSlicePod<T> for [Volatile<T>] {
    fn map_slice<U: Pod>(&self) -> Option<&[Volatile<U>]> {
        unsafe { Pod::map_slice(unwrap_as_slice(self)).map(|x| Volatile::slice_from_ref(x)) }
    }

    fn merge<U: Pod>(&self) -> Option<&Volatile<U>> {
        unsafe { Pod::merge(unwrap_as_slice(self)).map(|x| Volatile::from_ref(x)) }
    }

    fn copy_to_slice(&self, slice: &mut [T]) {
        assert_eq!(self.len(), slice.len());
        for (x, y) in self.iter().zip(slice) {
            *y = x.load();
        }
    }

    fn copy_from_slice(&self, slice: &[T]) {
        assert_eq!(self.len(), slice.len());
        for (x, y) in self.iter().zip(slice) {
            x.store(y.copy());
        }
    }
}

impl<T: Pod + fmt::Debug> fmt::Debug for Volatile<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Volatile").field(&self.load()).finish()
    }
}

impl<T: Pod> Clone for Volatile<T> {
    fn clone(&self) -> Self {
        Self::new(self.load())
    }
}

/// `volatile_view` prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use super::{VolatilePod, VolatileSlicePod};
}
