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

            fn new_private() -> $private_type {
                $private_type {
                    $(
                    $interface_ident: <$interface_type>::from_vtable(&*$vtable as *const $vtable_type),
                    )*
                    ref_count: $crate::detail::AtomicIsize::new(1)
                }
            }
        }
        impl $crate::IUnknownTrait for $obj_type {
            unsafe fn query_interface(this: *mut Self, iid: &$crate::IID, object: *mut *mut ::std::os::raw::c_void) -> $crate::HResult {
                $(
                    if <$interface_type>::scan_iid(iid) {
                        $crate::IUnknownTrait::add_ref(this);
                        *object = &mut (*this).com_private.$interface_ident as *mut $interface_type as *mut ::std::os::raw::c_void;
                        $crate::E_OK
                    } else
                )* {
                    $crate::E_NOINTERFACE
                }
            }
            unsafe fn add_ref(this: *mut Self) -> u32 {
                let orig_ref_count = (*this).com_private.ref_count.fetch_add(1, $crate::detail::Ordering::Relaxed);
                (orig_ref_count + 1) as u32
            }
            unsafe fn release(this: *mut Self) -> u32 {
                let orig_ref_count = (*this).com_private.ref_count.fetch_sub(1, $crate::detail::Ordering::Release);
                assert!(orig_ref_count > 0);
                if orig_ref_count == 1 {
                    $crate::detail::fence($crate::detail::Ordering::Acquire);
                    $crate::detail::delete_obj_raw(this);
                }
                (orig_ref_count - 1) as u32
            }
        }
    )
}
