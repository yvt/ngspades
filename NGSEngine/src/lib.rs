//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[macro_use]
extern crate ngscom;

#[macro_use]
extern crate lazy_static;

use std::mem;
use ngscom::{IUnknown, IUnknownTrait, BString, BStringRef, HResult, ComPtr, E_OK};

iid!(IID_ITESTINTERFACE =
    0x35edff15, 0x0b38, 0x47d8, 0x9b, 0x7c, 0xe0, 0x0f, 0xa2, 0xac, 0xdf, 0x9d);

com_interface! {
    interface (ITestInterface, ITestInterfaceTrait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE,
        vtable: ITestInterfaceVTable,
        thunk: ITestInterfaceThunk,

        fn get_hoge_attr(retval: &mut BStringRef) -> HResult;
        fn set_hoge_attr(value: &BString) -> HResult;
        fn hello(value: &BString, retval: &mut BStringRef) -> HResult;
        fn simple_method() -> HResult;
    }
}

com_impl! {
    #[derive(Debug)]
    class TestClass {
        com_private: TestClassPrivate;
        itestinterface: (ITestInterface, ITestInterfaceVTable, TESTCLASS_VTABLE);
    }
}

impl ITestInterfaceTrait for TestClass {
    unsafe fn get_hoge_attr(_: *mut Self, retval: &mut BStringRef) -> HResult {
        *retval = BStringRef::new("You successfully GetHogeAttr'd!");
        E_OK
    }
    unsafe fn set_hoge_attr(_: *mut Self, value: &BString) -> HResult {
        println!("SetHogeAttr: I'm getting this: {:?}", value);
        E_OK
    }
    unsafe fn hello(_: *mut Self, value: &BString, retval: &mut BStringRef) -> HResult {
        println!("Hello! (got {:?})", value);
        println!("BString addr: {:x}, data: {:x}",
            mem::transmute::<_, usize>(value), mem::transmute::<_, usize>(&value.data()[0]));
        *retval = BStringRef::new("hOI! \0(null character here)");
        println!("Returning {:?}", retval);
        E_OK
    }
    unsafe fn simple_method(_: *mut Self) -> HResult {
        E_OK
    }
}

impl TestClass {
    fn new() -> ComPtr<ITestInterface> {
        ComPtr::from(&TestClass::alloc(TestClass{
            com_private: Self::new_private()
        }).0)
    }
}

#[no_mangle]
pub unsafe extern fn create_test_instance(retval: &mut ComPtr<ITestInterface>) -> HResult {
    *retval = TestClass::new();
    E_OK
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
