//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

/*
 * The lazy_static crate is required to use this macro
 * (unless #[macro_reexport] goes into a stable state):
 *
 *    #[macro_use] extern crate ngscom;
 *    #[macro_use] extern crate lazy_static;
 *    com_impl!(TESTCLASS_VTABLE, ITestIterface, TestClass);
 */

#[macro_export]
macro_rules! com_vtable {
    ( $vtable:ident, $vtable_type: ty, $interface_type:ty, $obj_type:ty ) => (
        lazy_static! {
            static ref $vtable: $vtable_type =
                <$interface_type>::fill_vtable::<$obj_type, $crate::StaticZeroOffset>();
        }
    )
}

#[macro_export]
macro_rules! com_impl {
    (
        $(#[$iface_attr:meta])*
        class $obj_type:ident {
            com_private: $private_type:ident;
            $(
                $interface_ident:ident : ($interface_type:ty, $vtable_type:ty, $vtable:ident)
            ),* ;
            $($rest:tt)*
        }
    ) => (
        $(
            // TODO: support non-zero offset for thunk functions
            // (currently, we cannot have more than one base interface)
            com_vtable!($vtable, $vtable_type, $interface_type, $obj_type);
        )*
        #[derive(Debug)]
        #[doc(hidden)]
        struct $private_type {
            $( $interface_ident: $interface_type, )*
            ref_count: $crate::detail::AtomicIsize,
        }
        $(#[$iface_attr])*
        pub struct $obj_type {
            com_private: $private_type,
            $($rest)*
        }
        impl $obj_type {
            fn alloc(x: $obj_type) -> ($crate::ComPtr<$crate::IUnknown>, *mut Self) {
                let ptr = $crate::detail::new_obj_raw(x);
                let mut comptr: $crate::ComPtr<$crate::IUnknown> = ComPtr::new();
                (*comptr.as_mut_ptr()) = ptr as *mut $crate::IUnknown;
                ( comptr, ptr )
            }

            #[doc(hidden)]
            fn new_private() -> $private_type {
                $private_type {
                    $(
                    $interface_ident: <$interface_type>::from_vtable(&*$vtable as *const $vtable_type),
                    )*
                    ref_count: $crate::detail::AtomicIsize::new(1)
                }
            }
        }
        // It's safe to implement Sync because the contents of vtable
        // doesn't actually change
        unsafe impl ::std::marker::Sync for $private_type {}
        impl ::std::default::Default for $private_type {
            fn default() -> Self {
                $obj_type::new_private()
            }
        }
        impl $crate::IUnknownTrait for $obj_type {
            fn query_interface(&self, iid: &$crate::IID, object: *mut *mut ::std::os::raw::c_void) -> $crate::HResult {
                $(
                    if <$interface_type>::scan_iid(iid) {
                        unsafe {
                            $crate::IUnknownTrait::add_ref(self);
                            *object = &self.com_private.$interface_ident
                                as *const $interface_type as *mut $interface_type
                                as *mut ::std::os::raw::c_void;
                        }
                        $crate::hresults::E_OK
                    } else
                )* {
                    $crate::hresults::E_NOINTERFACE
                }
            }
            fn add_ref(&self) -> u32 {
                let orig_ref_count = self.com_private.ref_count.fetch_add(1, $crate::detail::Ordering::Relaxed);
                if orig_ref_count == ::std::isize::MAX {
                    // FIXME: poison the object?
                    panic!("ref count overflowed");
                }
                (orig_ref_count + 1) as u32
            }
            unsafe fn release(&self) -> u32 {
                let orig_ref_count = self.com_private.ref_count.fetch_sub(1, $crate::detail::Ordering::Release);
                assert!(orig_ref_count > 0);
                if orig_ref_count == 1 {
                    $crate::detail::fence($crate::detail::Ordering::Acquire);
                    $crate::detail::delete_obj_raw(self);
                }
                (orig_ref_count - 1) as u32
            }
        }
    )
}
