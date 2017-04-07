#[macro_use]
extern crate ngscom;

#[macro_use]
extern crate lazy_static;

use ngscom::{IUnknown, IUnknownTrait, ComPtr};

iid!(IID_ITESTINTERFACE1 =
    0x35edff15, 0x0b38, 0x47d8, 0x9b, 0x7c, 0xe0, 0x0f, 0xa2, 0xac, 0xdf, 0x9d);

com_interface! {
    interface (ITestInterface1, ITestInterface1Trait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE1,
        vtable: ITestInterface1VTable,
        thunk: ITestInterface1Thunk,

        fn get_hoge_attr1() -> i32;
        fn set_hoge_attr1(value: i32) -> ();
    }
}

iid!(IID_ITESTINTERFACE2 =
    0xee62c096, 0xc18e, 0x467d, 0x85, 0x79, 0xde, 0x07, 0x62, 0xaf, 0xfa, 0xe6);

com_interface! {
    interface (ITestInterface2, ITestInterface2Trait): (ITestInterface1, ITestInterface1Trait), IUnknown {
        iid: IID_ITESTINTERFACE2,
        vtable: ITestInterface2VTable,
        thunk: ITestInterface2Thunk,

        fn get_hoge_attr2() -> i32;
        fn set_hoge_attr2(value: i32) -> ();
    }
}

com_impl! {
    #[derive(Debug)]
    class TestClass {
        com_private: TestClassPrivate;
        itestinterface2: (ITestInterface2, ITestInterface2VTable, TESTCLASS_VTABLE);
        test_field1: i32,
        test_field2: i32,
    }
}

impl ITestInterface1Trait for TestClass {
    unsafe fn get_hoge_attr1(this: *mut Self) -> i32 {
        (*this).test_field1
    }
    unsafe fn set_hoge_attr1(this: *mut Self, value: i32) {
        (*this).test_field1 = value;
    }
}

impl ITestInterface2Trait for TestClass {
    unsafe fn get_hoge_attr2(this: *mut Self) -> i32 {
        (*this).test_field2
    }
    unsafe fn set_hoge_attr2(this: *mut Self, value: i32) {
        (*this).test_field2 = value;
    }
}

impl TestClass {
    fn new(num: i32) -> ComPtr<ITestInterface2> {
        ComPtr::from(&TestClass::alloc(TestClass{
            com_private: Self::new_private(),
            test_field1: num,
            test_field2: num,
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
        inst.set_hoge_attr1(114);
        inst.set_hoge_attr2(514);
        assert_eq!(inst.get_hoge_attr1(), 114);
        assert_eq!(inst.get_hoge_attr2(), 514);
    }
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