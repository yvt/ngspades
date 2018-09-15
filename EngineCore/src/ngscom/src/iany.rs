//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use crate::{IUnknown, IUnknownTrait};
use std::any::Any;

com_iid!(
    IID_IANY = [
        0xcae77653,
        0x6042,
        0x48b8,
        [0x82, 0xdc, 0x92, 0x6d, 0xec, 0x0e, 0x34, 0x49]
    ]
);

com_interface! {
    /// Provides an interface to `std::any::Any`.
    interface (IAny, IAnyTrait): (IUnknown, IUnknownTrait) {
        iid: IID_IANY,
        vtable: IAnyVTable,

        fn get_any() -> *const (dyn Any + Send + Sync);
    }
}

impl IAny {
    /// Return `true` if the actual type is the same as `T`.
    pub fn is<T: Any>(&self) -> bool {
        Any::is::<T>(unsafe { &*self.get_any() })
    }

    /// Return a reference to the inner value if it is of type `T`, or `None` otherwise.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        Any::downcast_ref(unsafe { &*self.get_any() })
    }
}

impl<T: Any + Send + Sync + IUnknownTrait> IAnyTrait for T {
    fn get_any(&self) -> *const (dyn Any + Send + Sync) {
        self as &(dyn Any + Send + Sync)
    }
}
