//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ngscom::{
    com_iid, com_impl, com_interface, hresults, ComPtr, HResult, IUnknown, IUnknownTrait,
    UnownedComPtr,
};
use std::sync::{Arc, Mutex};

com_iid!(
    IID_ITESTINTERFACE = [
        0x35edff15,
        0x0b38,
        0x47d8,
        [0x9b, 0x7c, 0xe0, 0x0f, 0xa2, 0xac, 0xdf, 0x9d]
    ]
);

com_interface! {
    interface (ITestInterface, ITestInterfaceTrait): (IUnknown, IUnknownTrait) {
        iid: IID_ITESTINTERFACE,
        vtable: ITestInterfaceVTable,

        fn get_hoge_attr(retval: &mut ComPtr<ITestInterface>) -> HResult;
        fn set_hoge_attr(value: UnownedComPtr<'_, ITestInterface>) -> HResult;
    }
}

com_impl! {
    #[derive(Debug)]
    class TestClass {
        itestinterface: ITestInterface;
        @data: TestClassData;
    }
}

#[derive(Debug)]
struct TestClassData {
    test_field: Mutex<ComPtr<ITestInterface>>,
    state: Arc<Mutex<bool>>,
}

impl ITestInterfaceTrait for TestClass {
    fn get_hoge_attr(&self, retval: &mut ComPtr<ITestInterface>) -> HResult {
        let field = self.data.test_field.lock().unwrap();
        *retval = field.clone();
        hresults::E_OK
    }
    fn set_hoge_attr(&self, value: UnownedComPtr<'_, ITestInterface>) -> HResult {
        let mut field = self.data.test_field.lock().unwrap();
        *field = value.clone();
        hresults::E_OK
    }
}

impl ::std::ops::Drop for TestClassData {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap();
        *state = false;
    }
}

impl TestClass {
    fn new(state: Arc<Mutex<bool>>) -> ComPtr<ITestInterface> {
        {
            let mut s = state.lock().unwrap();
            *s = true;
        }
        ComPtr::from(&Self::alloc(TestClassData {
            test_field: Mutex::new(ComPtr::null()),
            state: state,
        }))
    }
}

#[test]
fn create_instance() {
    TestClass::new(Arc::new(Mutex::new(false)));
}

#[test]
fn access_field() {
    let alive_state = Arc::new(Mutex::new(false));
    assert_eq!(*alive_state.lock().unwrap(), false);
    {
        let inst = TestClass::new(alive_state.clone());
        let mut ret = ComPtr::null();
        assert!(!inst.is_null());

        assert_eq!(*alive_state.lock().unwrap(), true);

        inst.get_hoge_attr(&mut ret).unwrap();
        assert_eq!(ret.is_null(), true);

        inst.set_hoge_attr(UnownedComPtr::from_comptr(&inst))
            .unwrap();

        inst.get_hoge_attr(&mut ret).unwrap();
        assert_eq!(ret.is_null(), false);

        inst.set_hoge_attr(UnownedComPtr::null()).unwrap();
    }
    assert_eq!(*alive_state.lock().unwrap(), false);
}

#[test]
fn leak() {
    let alive_state = Arc::new(Mutex::new(false));
    assert_eq!(*alive_state.lock().unwrap(), false);
    {
        let inst = TestClass::new(alive_state.clone());
        let mut ret = ComPtr::null();
        assert!(!inst.is_null());

        assert_eq!(*alive_state.lock().unwrap(), true);

        inst.get_hoge_attr(&mut ret).unwrap();
        assert_eq!(ret.is_null(), true);

        inst.set_hoge_attr(UnownedComPtr::from_comptr(&inst))
            .unwrap();

        inst.get_hoge_attr(&mut ret).unwrap();
        assert_eq!(ret.is_null(), false);

        // a circular reference exists - leak occurs
    }
    assert_eq!(*alive_state.lock().unwrap(), true);
}
