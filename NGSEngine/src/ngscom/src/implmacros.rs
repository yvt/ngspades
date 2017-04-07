/*
 * The lazy_static crate is required to use this macro:
 *
 *    #[macro_use] extern crate ngscom;
 *    #[macro_use] extern crate lazy_static;
 *    com_impl!(TESTCLASS_VTABLE, ITestIterface, TestClass);
 */
#[macro_export]
macro_rules! com_impl {
    ( $vtable:ident, $vtable_type: ty, $interface_type:ty, $obj_type:ty ) => (
        lazy_static! {
            static ref $vtable: $vtable_type =
                unsafe { <$interface_type>::fill_vtable::<$obj_type, $crate::StaticZeroOffset>() };
        }
    )
}

