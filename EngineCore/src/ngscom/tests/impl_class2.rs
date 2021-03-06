//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Tests an interface with a two-level inheritance hierarchy
use ngscom::{com_iid, com_impl, com_interface, ComPtr, IUnknown, IUnknownTrait};
use std::sync::Mutex;

com_iid!(
    IID_ITESTINTERFACE1 = [
        0x35edff15,
        0x0b38,
        0x47d8,
        [0x9b, 0x7c, 0xe0, 0x0f, 0xa2, 0xac, 0xdf, 0x9d]
    ]
);

com_interface! {
    interface (ITestInterface1, ITestInterface1Trait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE1,
        vtable: ITestInterface1VTable,

        fn get_hoge_attr1() -> i32;
        fn set_hoge_attr1(value: i32) -> ();
    }
}

com_iid!(
    IID_ITESTINTERFACE2 = [
        0xee62c096,
        0xc18e,
        0x467d,
        [0x85, 0x79, 0xde, 0x07, 0x62, 0xaf, 0xfa, 0xe6]
    ]
);

com_interface! {
    interface (ITestInterface2, ITestInterface2Trait): (ITestInterface1, ITestInterface1Trait), IUnknown {
        iid: IID_ITESTINTERFACE2,
        vtable: ITestInterface2VTable,

        fn get_hoge_attr2() -> i32;
        fn set_hoge_attr2(value: i32) -> ();
    }
}

com_impl! {
    #[derive(Debug)]
    class TestClass {
        itestinterface2: ITestInterface2;
        @data: TestClassData;
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
    fn new(num: i32) -> ComPtr<ITestInterface2> {
        ComPtr::from(&Self::alloc(TestClassData {
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
fn access_field() {
    let inst = TestClass::new(114514);
    assert!(!inst.is_null());
    inst.set_hoge_attr1(114);
    inst.set_hoge_attr2(514);
    assert_eq!(inst.get_hoge_attr1(), 114);
    assert_eq!(inst.get_hoge_attr2(), 514);
}

#[test]
fn super_cast() {
    let inst = TestClass::new(114514);
    assert!(!inst.is_null());

    let ptr1: ComPtr<IUnknown> = ComPtr::from(&inst);
    let ptr2: ComPtr<ITestInterface1> = ComPtr::from(&inst);
    assert!(!ptr1.is_null());
    assert!(!ptr2.is_null());
}
