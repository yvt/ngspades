//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::{borrow, fmt, marker, mem, ops, ptr};
use std::raw::TraitObject;

/// Stores unsized data without extra heap allocation.
///
/// **`T` must specify a trait object type like `dyn SomeTrait`**. Using other
/// types results in an undefined behavior.
/// This restriction cannot be enforced by trait bounds applied to
/// `SmallBox::new` and therefore it is a developer's responsibility to ensure
/// this restriction is fulfilled.
/// To prevent forming an invalid instance of `SmallBox`, `SmallBox::new` is
/// marked as `unsafe`.
///
/// The size of data that can be stored in a single `SmallBox` is limited to
/// `size_of::<S>()`. Furthermore, the alignment requirement of the
/// contained data must not be stricter than that of `S`. `SmallBox::new`
/// will panic if any of these restrictions are violated.
///
/// # Examples
///
///     use std::fmt;
///     use zangfx_common::SmallBox;
///     let value = "hoge";
///     let boxed = unsafe { SmallBox::<dyn fmt::Debug, [usize; 2]>::new(value) };
///     assert_eq!(format!("{:?}", boxed), format!("{:?}", value));

pub struct SmallBox<T: ?Sized, S: Copy> {
    vtable: *mut (),
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
    /// Construct a `SmallBox` containing the given `S` (sized type) value.
    ///
    /// Panics if the value does not fit `SmallBox`. (There is no known way to
    /// check this in compile-time)
    pub unsafe fn new<C>(x: C) -> Self
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

        // Retrieve vtable
        let vtable = {
            let tobj: &T = &x;
            let tobj_raw: TraitObject = mem::transmute_copy(&tobj);
            tobj_raw.vtable
        };

        // Move the contents
        let mut storage: Storage<S> = mem::uninitialized();
        ptr::write(&mut storage as *mut Storage<S> as *mut C, x);

        Self {
            vtable,
            storage,
            _phantom: marker::PhantomData,
        }
    }

    fn trait_object(&self) -> TraitObject {
        TraitObject {
            vtable: self.vtable,
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn new() {
        unsafe {
            SmallBox::<dyn fmt::Debug, [usize; 2]>::new("hoge");
        }
    }

    #[test]
    fn debug() {
        let base_val = "hoge";
        let boxed = unsafe { SmallBox::<dyn fmt::Debug, [usize; 2]>::new(base_val) };
        assert_eq!(format!("{:?}", boxed), format!("{:?}", base_val));
    }

    #[test]
    fn drop() {
        use std::rc::Rc;
        let base_val = Rc::new(());
        assert_eq!(Rc::strong_count(&base_val), 1);
        {
            let _boxed =
                unsafe { SmallBox::<dyn fmt::Debug, [usize; 2]>::new(Rc::clone(&base_val)) };
            assert_eq!(Rc::strong_count(&base_val), 2);
        }
        assert_eq!(Rc::strong_count(&base_val), 1);
    }
}
