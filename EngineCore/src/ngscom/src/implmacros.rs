//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#[macro_export]
#[doc(hidden)]
macro_rules! com_vtable {
    ($vtable:ident, $vtable_type:ty, $interface_type:ty, $cls_type:ty, $offs:expr) => {
        $crate::lazy_static! {
            static ref $vtable: $vtable_type = {
                struct Offset;
                impl $crate::StaticOffset for Offset {
                    fn offset() -> isize {
                        $offs
                    }
                }
                <$interface_type>::fill_vtable::<$cls_type, Offset>()
            };
        }
    };
}

/// Macro for defining a COM class.
#[macro_export]
macro_rules! com_impl {
    (
        $(#[$cls_attr:meta])*
        class $cls_type:ident {
            $(
                $interface_ident:ident : $interface_type:ty;
            )*
            $(#[$data_attr:meta])*
            @data: $data:ty;
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
                            $crate::com_vtable!(
                                VTABLE, <$interface_type as $crate::ComInterface>::Vtable, $interface_type, $cls_type,
                                {
                                    // offset from `$interface_ident` to `Self`
                                    let p: *const $cls_type = ::std::ptr::null();
                                    unsafe {
                                        (p as isize).wrapping_sub(&(*p).$interface_ident as *const _ as isize)
                                    }
                                }
                            );

                            &*VTABLE
                        } as *const _),
                    )*
                    ref_count: $crate::detail::AtomicIsize::new(1),
                    data: x,
                });
                let mut comptr: $crate::ComPtr<$crate::IUnknown> = ComPtr::new();
                (*comptr.as_mut_ptr()) = ptr as *mut $crate::IUnknown;
                comptr
            }

            /// Construct a `ComPtr<IUnknown>` pointing `self`.
            #[allow(dead_code)]
            pub fn as_com_ptr(&self) -> $crate::ComPtr<$crate::IUnknown> {
                assert_ne!(self.ref_count.load($crate::detail::Ordering::Relaxed), 0, "can't resurrect an object");
                $crate::IUnknownTrait::add_ref(self);

                let mut comptr: $crate::ComPtr<$crate::IUnknown> = ComPtr::new();
                unsafe { (*comptr.as_mut_ptr::<$crate::IUnknown>()) = ::std::mem::transmute(self); }
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
