//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[macro_use]
extern crate ngscom;

#[macro_use]
extern crate lazy_static;

use ngscom::{IUnknown, IUnknownTrait, ComPtr, HResult, hresults};
use std::sync::Mutex;
use std::default::Default;

com_iid!(IID_ITESTINTERFACE =
    [0x35edff15, 0x0b38, 0x47d8, [0x9b, 0x7c, 0xe0, 0x0f, 0xa2, 0xac, 0xdf, 0x9d]]);

com_interface! {
    interface (ITestInterface, ITestInterfaceTrait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE,
        vtable: ITestInterfaceVTable,
        thunk: ITestInterfaceThunk,

        fn get_hoge_attr(retval: &mut i32) -> HResult;
        fn set_hoge_attr(value: i32) -> HResult;
    }
}

com_impl! {
    #[derive(Debug)]
    class TestClass {
        com_private: TestClassPrivate;
        itestinterface: (ITestInterface, ITestInterfaceVTable, TESTCLASS_VTABLE);
        test_field: Mutex<i32>
    }
}

impl ITestInterfaceTrait for TestClass {
    fn get_hoge_attr<'a>(&self, retval: &'a mut i32) -> HResult {
        let field = self.test_field.lock().unwrap();
        *retval = *field;
        hresults::E_OK
    }
    fn set_hoge_attr(&self, value: i32) -> HResult {
        let mut field = self.test_field.lock().unwrap();
        *field = value;
        hresults::E_OK
    }
}

impl TestClass {
    fn new(num: i32) -> ComPtr<ITestInterface> {
        ComPtr::from(&TestClass::alloc(TestClass{
            com_private: Default::default(),
            test_field: Mutex::new(num),
        }).0)
    }
}

#[test]
fn create_instance() {
    TestClass::new(114514);
}

#[test]
fn access_field() {
    let inst = TestClass::new(114514);
    assert!(!inst.is_null());
    inst.set_hoge_attr(42).unwrap();
    let mut value = 4 as i32;
    inst.get_hoge_attr(&mut value).unwrap();
    assert_eq!(value, 42);
}
