//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tests a class implementing multiple interfaces

#[macro_use]
extern crate ngscom;

#[macro_use]
extern crate lazy_static;

use ngscom::{IUnknown, IUnknownTrait, ComPtr};
use std::sync::Mutex;

com_iid!(IID_ITESTINTERFACE1 =
    [0x35edff15, 0x0b38, 0x47d8, [0x9b, 0x7c, 0xe0, 0x0f, 0xa2, 0xac, 0xdf, 0x9d]]);

com_interface! {
    interface (ITestInterface1, ITestInterface1Trait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE1,
        vtable: ITestInterface1VTable,

        fn get_hoge_attr1() -> i32;
        fn set_hoge_attr1(value: i32) -> ();
    }
}

com_iid!(IID_ITESTINTERFACE2 =
    [0xee62c096, 0xc18e, 0x467d, [0x85, 0x79, 0xde, 0x07, 0x62, 0xaf, 0xfa, 0xe6]]);

com_interface! {
    interface (ITestInterface2, ITestInterface2Trait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE2,
        vtable: ITestInterface2VTable,

        fn get_hoge_attr2() -> i32;
        fn set_hoge_attr2(value: i32) -> ();
    }
}

com_impl! {
    #[derive(Debug)]
    class TestClass {
        itestinterface1: (ITestInterface1, ITestInterface1VTable),
        itestinterface2: (ITestInterface2, ITestInterface2VTable);
        data: TestClassData;
    }
}

#[derive(Debug)]
struct TestClassData {
    test_field1: Mutex<i32>,
    test_field2: Mutex<i32>,
}

impl ITestInterface1Trait for TestClass {
    fn get_hoge_attr1(&self) -> i32 {
        let field = self.data.test_field1.lock().unwrap();
        *field
    }
    fn set_hoge_attr1(&self, value: i32) {
        let mut field = self.data.test_field1.lock().unwrap();
        *field = value;
    }
}

impl ITestInterface2Trait for TestClass {
    fn get_hoge_attr2(&self) -> i32 {
        let field = self.data.test_field2.lock().unwrap();
        *field
    }
    fn set_hoge_attr2(&self, value: i32) {
        let mut field = self.data.test_field2.lock().unwrap();
        *field = value;
    }
}

impl TestClass {
    fn new(num: i32) -> ComPtr<ITestInterface1> {
        ComPtr::from(&Self::alloc(TestClassData{
            test_field1: Mutex::new(num),
            test_field2: Mutex::new(num),
        }))
    }
}

#[test]
fn create_instance() {
    TestClass::new(114514);
}

#[test]
fn cast() {
    let inst_1 = TestClass::new(114514);
    let inst_2 = ComPtr::<ITestInterface2>::from(&inst_1);
    assert!(!inst_1.is_null());
    assert!(!inst_2.is_null());

    // convert back
    let inst_3 = ComPtr::<ITestInterface1>::from(&inst_2);
    assert!(!inst_3.is_null());
}

#[test]
fn access_field() {
    let inst_1 = TestClass::new(114514);
    let inst_2 = ComPtr::<ITestInterface2>::from(&inst_1);
    inst_1.set_hoge_attr1(114);
    inst_2.set_hoge_attr2(514);
    assert_eq!(inst_1.get_hoge_attr1(), 114);
    assert_eq!(inst_2.get_hoge_attr2(), 514);
}
