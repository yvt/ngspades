//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::raw::TraitObject;
use std::{borrow, fmt, marker, mem, ops, ptr};

/// Stores unsized data without extra heap allocation.
///
/// **`T` must specify a trait object type like `dyn SomeTrait`**. Using other
/// types results in a panic.
/// This restriction cannot be enforced by trait bounds applied to
/// `SmallBox::new` and therefore it is a developer's responsibility to ensure
/// this restriction is fulfilled.
///
/// The size of data that can be stored in a single `SmallBox` is limited to
/// `size_of::<S>()`. Furthermore, the alignment requirement of the
/// contained data must not be stricter than that of `S`. `SmallBox::new`
/// will panic if any of these restrictions are violated.
///
/// # Examples
///
/// ```
/// use std::fmt;
/// use zangfx_common::SmallBox;
/// let value = "hoge";
/// let boxed = SmallBox::<dyn fmt::Debug, [usize; 2]>::new(value);
/// assert_eq!(format!("{:?}", boxed), format!("{:?}", value));
/// ```
///
/// `T` must be a trait object. Otherwise `new` will fail at runtime:
///
/// ```should_panic
/// # use zangfx_common::SmallBox;
/// // `[usize]` is `!Sized` but not a trait object, so
/// // the following code will cause a panic
/// SmallBox::<[usize], [usize; 2]>::new([1, 2]);
/// ```
///
pub struct SmallBox<T: ?Sized, S: Copy> {
    vtable: ptr::NonNull<()>,
    storage: Storage<S>,
    _phantom: marker::PhantomData<T>,
}

unsafe impl<T: ?Sized + Sync, S: Copy> Sync for SmallBox<T, S> {}
unsafe impl<T: ?Sized + Send, S: Copy> Send for SmallBox<T, S> {}

#[repr(C)]
union Storage<S: Copy> {
    _dummy: S,
}

impl<T: ?Sized, S: Copy> SmallBox<T, S> {
    /// Construct a `SmallBox` containing the given `C` (sized type) value.
    ///
    /// Panics if the value does not fit `SmallBox`. (There is no known way to
    /// check this in compile-time)
    pub fn new<C>(x: C) -> Self
    where
        C: marker::Unsize<T>,
    {
        assert!(
            mem::size_of::<C>() <= mem::size_of::<Storage<S>>(),
            "C is too large to store in SmallBox. ({} > {})",
            mem::size_of::<C>(),
            mem::size_of::<Storage<S>>(),
        );
        assert!(
            mem::align_of::<C>() <= mem::align_of::<Storage<S>>(),
            "The alignment requirement of C is too strict ({} > {}).",
            mem::align_of::<C>(),
            mem::align_of::<Storage<S>>(),
        );

        use metatype::{MetaType, Type};
        assert_eq!(
            <T as Type>::METATYPE,
            MetaType::TraitObject,
            "T is not a trait object"
        );

        // Retrieve vtable
        let vtable = unsafe {
            let tobj: &T = &x;
            let tobj_raw: TraitObject = mem::transmute_copy(&tobj);
            tobj_raw.vtable
        };

        let vtable = ptr::NonNull::new(vtable).unwrap();

        // Move the contents
        unsafe {
            let mut storage: Storage<S> = mem::uninitialized();
            ptr::write(&mut storage as *mut Storage<S> as *mut C, x);

            Self {
                vtable,
                storage,
                _phantom: marker::PhantomData,
            }
        }
    }

    /// Construct a `SmallBox` containing the given `C` (sized type) value.
    ///
    /// The returned box supports [`SmallBox::downcast_ref`] and other related
    /// operations.
    ///
    /// Panics if the value does not fit `SmallBox`. (There is no known way to
    /// check this in compile-time)
    pub fn new_downcastable<C>(x: C) -> Self
    where
        C: StableVtable<T>,
    {
        assert!(
            mem::size_of::<C>() <= mem::size_of::<Storage<S>>(),
            "C is too large to store in SmallBox. ({} > {})",
            mem::size_of::<C>(),
            mem::size_of::<Storage<S>>(),
        );
        assert!(
            mem::align_of::<C>() <= mem::align_of::<Storage<S>>(),
            "The alignment requirement of C is too strict ({} > {}).",
            mem::align_of::<C>(),
            mem::align_of::<Storage<S>>(),
        );

        // Move the contents
        unsafe {
            let mut storage: Storage<S> = mem::uninitialized();
            ptr::write(&mut storage as *mut Storage<S> as *mut C, x);

            Self {
                vtable: C::stable_vtable(),
                storage,
                _phantom: marker::PhantomData,
            }
        }
    }

    /// Return `true` if the boxed type is the same as `C`. False negative can
    /// occur if the box was not constructed by [`SmallBox::new_downcastable`].
    pub fn is<C>(&self) -> bool
    where
        C: StableVtable<T>,
    {
        self.vtable.as_ptr() == C::stable_vtable_or_null()
    }

    /// Get a reference of the boxed value if it's of type `C`.
    ///
    /// Not to mention that the concrete type of the box must match `C`, the
    /// following conditions must be met in addition to guarantee a successful
    /// downcast:
    ///
    ///  - `C` must implement [`StableVtable`].
    ///  - `self` was constructed using [`SmallBox::new_downcastable`].
    ///
    pub fn downcast_ref<C>(&self) -> Option<&C>
    where
        C: StableVtable<T>,
    {
        if self.is::<C>() {
            unsafe { Some(&*(&self.storage as *const Storage<S> as *const C)) }
        } else {
            None
        }
    }

    /// Get a mutable reference of the boxed value if it's of type `C`.
    ///
    /// Not to mention that the concrete type of the box must match `C`, the
    /// following conditions must be met in addition to guarantee a successful
    /// downcast:
    ///
    ///  - `C` must implement [`StableVtable`].
    ///  - `self` was constructed using [`SmallBox::new_downcastable`].
    ///
    pub fn downcast_mut<C>(&mut self) -> Option<&mut C>
    where
        C: StableVtable<T>,
    {
        if self.is::<C>() {
            unsafe { Some(&mut *(&mut self.storage as *mut Storage<S> as *mut C)) }
        } else {
            None
        }
    }

    fn trait_object(&self) -> TraitObject {
        TraitObject {
            vtable: self.vtable.as_ptr(),
            data: &self.storage as *const Storage<S> as *const () as *mut (),
        }
    }
}

impl<T: ?Sized, S: Copy> ops::Deref for SmallBox<T, S> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { mem::transmute_copy(&self.trait_object()) }
    }
}

impl<T: ?Sized, S: Copy> ops::DerefMut for SmallBox<T, S> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { mem::transmute_copy(&self.trait_object()) }
    }
}

impl<T: ?Sized, S: Copy> borrow::Borrow<T> for SmallBox<T, S> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized, S: Copy> borrow::BorrowMut<T> for SmallBox<T, S> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: ?Sized, S: Copy> AsRef<T> for SmallBox<T, S> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: ?Sized, S: Copy> AsMut<T> for SmallBox<T, S> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: ?Sized, S: Copy> Drop for SmallBox<T, S> {
    fn drop(&mut self) {
        unsafe { ptr::drop_in_place(&mut **self) };
    }
}

impl<T: ?Sized + fmt::Debug, S: Copy> fmt::Debug for SmallBox<T, S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (**self).fmt(f)
    }
}

/// Provides a stable vtable pointer for a specific trait object type `T`
/// (`dyn SomeTrait`).
///
/// Use the macro [`impl_stable_vtable`] to implement this trait on a
/// non-generic.
pub unsafe trait StableVtable<T: ?Sized>: marker::Unsize<T> + 'static {
    /// Get a stable vtable pointer for a specific trait object type `T`
    /// (`dyn SomeTrait`).
    ///
    /// The returned value must be derived from
    /// [`std::raw::TraitObject::vtable`].
    /// There must be an unique injective mapping from `T` to the returned
    /// value. This means:
    ///
    ///  - The returned value must be stable.
    ///  - No two distinct types may return an identical value.
    ///
    fn stable_vtable() -> std::ptr::NonNull<()>
    where
        Self: Sized;

    /// Do the same as `stable_vtable` except that it may return a null pointer
    /// if `stable_vtable` hasn't yet been called throughout the lifetime of
    /// the program.
    ///
    /// This method is used to optimize downcast operations.
    fn stable_vtable_or_null() -> *mut ()
    where
        Self: Sized,
    {
        Self::stable_vtable().as_ptr()
    }
}

/// Implements `StableVtable` on a non-generic type.
///
/// # Examples
///
///     use std::fmt::Debug;
///
///     #[derive(Debug)] struct Hoge;
///     zangfx_common::impl_stable_vtable! {
///         impl StableVtable<dyn Debug> for Hoge
///     }
///
#[macro_export]
macro_rules! impl_stable_vtable {
    (impl StableVtable<$traitobj:ty> for $type:ty) => {
        impl $type {
            /// Return a reference to the storage where the canonical return
            /// value of `stable_vtable` for a specific type is stored.
            ///
            /// We can't just return `TraitObject::vtable` of an imaginary
            /// reference to `$type` because we observed that it might change
            /// across compilation units.
            #[inline]
            #[doc(hidden)]
            fn __vtable_cell() -> &'static std::sync::atomic::AtomicUsize {
                use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};
                static VTABLE_CELL: AtomicUsize = ATOMIC_USIZE_INIT;
                &VTABLE_CELL
            }

            #[inline(never)]
            #[doc(hidden)]
            fn __stable_vtable_slow() -> std::ptr::NonNull<()> {
                use std::mem::transmute;
                use std::sync::atomic::Ordering;
                use $crate::metatype::{TraitObject, Type};

                let vtable_cell = Self::__vtable_cell();

                // This method was called for the first time.
                // Compute the vtable pointer.
                let dummy_concrete = unsafe { &*(1 as *const $type) };
                let dummy_to = dummy_concrete as &$traitobj;

                let meta: TraitObject = unsafe { transmute(Type::meta(dummy_to)) };

                let mut vtable = meta.vtable as *const _ as usize;

                assert_ne!(vtable, 0);

                // Make sure there exists exactly one return value of
                // `stable_vtable` for this specific type
                match vtable_cell.compare_and_swap(0, vtable, Ordering::Relaxed) {
                    0 => {}
                    x => {
                        vtable = x;
                    }
                }

                unsafe { std::ptr::NonNull::new_unchecked(vtable as *mut ()) }
            }
        }

        unsafe impl $crate::StableVtable<$traitobj> for $type {
            #[inline]
            fn stable_vtable_or_null() -> *mut () {
                use std::sync::atomic::Ordering;
                Self::__vtable_cell().load(Ordering::Relaxed) as *mut ()
            }

            #[inline]
            fn stable_vtable() -> std::ptr::NonNull<()> {
                std::ptr::NonNull::new(Self::stable_vtable_or_null())
                    .unwrap_or_else(|| Self::__stable_vtable_slow())
            }
        }
    };
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new() {
        SmallBox::<dyn fmt::Debug, [usize; 2]>::new("hoge");
    }

    #[test]
    fn debug() {
        let base_val = "hoge";
        let boxed = SmallBox::<dyn fmt::Debug, [usize; 2]>::new(base_val);
        assert_eq!(format!("{:?}", boxed), format!("{:?}", base_val));
    }

    #[test]
    fn drop() {
        use std::rc::Rc;
        let base_val = Rc::new(());
        assert_eq!(Rc::strong_count(&base_val), 1);
        {
            let _boxed = SmallBox::<dyn fmt::Debug, [usize; 2]>::new(Rc::clone(&base_val));
            assert_eq!(Rc::strong_count(&base_val), 2);
        }
        assert_eq!(Rc::strong_count(&base_val), 1);
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    struct Foo;

    impl_stable_vtable! {
        impl StableVtable<dyn fmt::Debug> for Foo
    }

    #[test]
    fn downcastable_debug() {
        let base_val = Foo;
        let boxed = SmallBox::<dyn fmt::Debug, [usize; 2]>::new_downcastable(base_val.clone());
        assert_eq!(format!("{:?}", boxed), format!("{:?}", base_val));
    }

    #[test]
    fn downcastable_downcast() {
        let boxed = SmallBox::<dyn fmt::Debug, [usize; 2]>::new_downcastable(Foo);
        assert_eq!(boxed.downcast_ref::<Foo>(), Some(&Foo));
    }
}
