//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[macro_use]
extern crate ngscom;
#[macro_use]
extern crate lazy_static;
extern crate ngsbase;

use std::mem;
use ngscom::{BString, BStringRef, HResult, ComPtr, hresults};
use ngsbase::{ITestInterface, ITestInterfaceTrait, ITestInterfaceVtbl};

com_impl! {
    #[derive(Debug)]
    class TestClass {
        com_private: TestClassPrivate;
        itestinterface: (ITestInterface, ITestInterfaceVtbl, TESTCLASS_VTABLE);
    }
}

impl ITestInterfaceTrait for TestClass {
    fn get_hoge_attr(&self, retval: &mut BStringRef) -> HResult {
        *retval = BStringRef::new("You successfully GetHogeAttr'd!");
        hresults::E_OK
    }
    fn set_hoge_attr(&self, value: Option<&BString>) -> HResult {
        println!("SetHogeAttr: I'm getting this: {:?}", value.unwrap());
        hresults::E_OK
    }
    fn hello(&self, value: Option<&BString>, retval: &mut BStringRef) -> HResult {
        println!("Hello! (got {:?})", value.unwrap());
        unsafe {
            println!("BString addr: {:x}, data: {:x}",
                     mem::transmute::<_, usize>(value),
                     mem::transmute::<_, usize>(&value.unwrap().data()[0]));
        }
        *retval = BStringRef::new("hOI! \0(null character here)");
        println!("Returning {:?}", retval);
        hresults::E_OK
    }
    fn simple_method(&self) -> HResult {
        hresults::E_OK
    }
}

impl TestClass {
    fn new() -> ComPtr<ITestInterface> {
        ComPtr::from(&TestClass::alloc(TestClass { com_private: Self::new_private() }).0)
    }
}

#[no_mangle]
pub unsafe extern "C" fn create_test_instance(retval: &mut ComPtr<ITestInterface>) -> HResult {
    *retval = TestClass::new();
    hresults::E_OK
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
