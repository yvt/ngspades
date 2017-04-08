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

iid!(IID_ITESTINTERFACE =
    0x35edff15, 0x0b38, 0x47d8, 0x9b, 0x7c, 0xe0, 0x0f, 0xa2, 0xac, 0xdf, 0x9d);

com_interface! {
    interface (ITestInterface, ITestInterfaceTrait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE,
        vtable: ITestInterfaceVTable,
        thunk: ITestInterfaceThunk,

        fn get_hoge_attr(retval: &mut i32) -> ();
        fn set_hoge_attr(value: i32) -> ();
    }
}

com_impl! {
    #[derive(Debug)]
    class TestClass {
        com_private: TestClassPrivate;
        itestinterface: (ITestInterface, ITestInterfaceVTable, TESTCLASS_VTABLE);
        test_field: i32
    }
}

impl ITestInterfaceTrait for TestClass {
    unsafe fn get_hoge_attr<'a>(this: *mut Self, retval: &'a mut i32) -> () {
        *retval = (*this).test_field;
    }
    unsafe fn set_hoge_attr(this: *mut Self, value: i32) {
        (*this).test_field = value;
    }
}

impl TestClass {
    fn new(num: i32) -> ComPtr<ITestInterface> {
        ComPtr::from(&TestClass::alloc(TestClass{
            com_private: Self::new_private(),
            test_field: num,
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
    unsafe {
        inst.set_hoge_attr(42);
        let mut value = 4 as i32;
        inst.get_hoge_attr(&mut value);
        assert_eq!(value, 42);
    }
}
