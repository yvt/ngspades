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
#[doc(hidden)]
macro_rules! com_vtable {
    ( $vtable:ident, $vtable_type: ty, $interface_type:ty, $cls_type:ty ) => (
        lazy_static! {
            static ref $vtable: $vtable_type =
                <$interface_type>::fill_vtable::<$cls_type, $crate::StaticZeroOffset>();
        }
    )
}

#[macro_export]
macro_rules! com_impl {
    (
        $(#[$cls_attr:meta])*
        class $cls_type:ident {
            $(
                $interface_ident:ident : ($interface_type:ty, $vtable_type:ty)
            ),* ;
            $(#[$data_attr:meta])*
            data: $data:ty;
        }
    ) => (
        $(#[$cls_attr])*
        pub struct $cls_type {
            _com_class_header: $crate::detail::ComClassHeader,
            $( $interface_ident: $interface_type, )*
            ref_count: $crate::detail::AtomicIsize,
            $(#[$data_attr])*
            data: $data,
        }
        impl $cls_type {
            fn alloc(x: $data) -> $crate::ComPtr<$crate::IUnknown> {
                let ptr = $crate::detail::new_obj_raw(Self {
                    _com_class_header: unsafe { $crate::detail::ComClassHeader::new() },
                    $(
                        $interface_ident: <$interface_type>::from_vtable({
                            // TODO: support non-zero offset for thunk functions
                            // (currently, we cannot have more than one base interface)
                            com_vtable!(VTABLE, $vtable_type, $interface_type, $cls_type);
                            &*VTABLE
                        } as *const $vtable_type),
                    )*
                    ref_count: $crate::detail::AtomicIsize::new(1),
                    data: x,
                });
                let mut comptr: $crate::ComPtr<$crate::IUnknown> = ComPtr::new();
                (*comptr.as_mut_ptr()) = ptr as *mut $crate::IUnknown;
                comptr
            }
        }

        impl $crate::IUnknownTrait for $cls_type {
            fn query_interface(&self, iid: &$crate::IID, object: *mut *mut ::std::os::raw::c_void) -> $crate::HResult {
                $(
                    if <$interface_type>::scan_iid(iid) {
                        unsafe {
                            $crate::IUnknownTrait::add_ref(self);
                            *object = &self.$interface_ident
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
                let orig_ref_count = self.ref_count.fetch_add(1, $crate::detail::Ordering::Relaxed);
                if orig_ref_count == ::std::isize::MAX {
                    // FIXME: poison the object?
                    panic!("ref count overflowed");
                }
                (orig_ref_count + 1) as u32
            }
            unsafe fn release(&self) -> u32 {
                let orig_ref_count = self.ref_count.fetch_sub(1, $crate::detail::Ordering::Release);
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
