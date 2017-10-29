//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

#[macro_use]
extern crate ngscom;

#[macro_use]
extern crate lazy_static;

use ngscom::{IUnknown, IUnknownTrait, ComPtr};
use std::sync::Mutex;

com_iid!(IID_ITESTINTERFACE =
    [0x35edff15, 0x0b38, 0x47d8, [0x9b, 0x7c, 0xe0, 0x0f, 0xa2, 0xac, 0xdf, 0x9d]]);

com_interface! {
    interface (ITestInterface, ITestInterfaceTrait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE,
        vtable: ITestInterfaceVTable,

        fn get_hoge_attr() -> i32;
        fn set_hoge_attr(value: i32) -> ();
    }
}

com_impl! {
    #[derive(Debug)]
    class TestClass {
        itestinterface: (ITestInterface, ITestInterfaceVTable);
        data: TestClassData;
    }
}

#[derive(Debug)]
struct TestClassData {
    test_field: Mutex<i32>,
}

impl ITestInterfaceTrait for TestClass {
    fn get_hoge_attr(&self) -> i32 {
        let field = self.data.test_field.lock().unwrap();
        *field
    }
    fn set_hoge_attr(&self, value: i32) {
        let mut field = self.data.test_field.lock().unwrap();
        *field = value;
    }
}

impl TestClass {
    fn new(num: i32) -> ComPtr<ITestInterface> {
        ComPtr::from(&Self::alloc(TestClassData{
            test_field: Mutex::new(num),
        }))
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
    inst.set_hoge_attr(42);
    assert_eq!(inst.get_hoge_attr(), 42);
}
